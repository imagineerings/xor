use comfy_model::clip_tokenizer::{
    CLIP_TOKENIZER_SOURCE_ROWS, apply_empty_baseline_token_weights, escape_important,
    generate_empty_tokens, parse_parentheses, token_weights, unescape_important,
};
use comfy_model::{
    ArtifactIndex, ArtifactKey, ArtifactRoot, ClipBpeTokenizer, ModelStore,
    ModelTokenizerDescriptor, NativePromptTokenizer, NativeTokenValue, NativeTokenizerError,
    NativeTokenizerFamily, ParserLimits, SentencePieceTokenizer, TextualInversionEmbedding,
    TokenizerConfiguration, parse_prompt_weights,
};
use comfy_tensor::CancellationToken;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

const TOKENIZER_IMPLEMENTATION_CLOSURE: [(&str, &str); 7] = [
    (
        "crates/comfy_model/src/clip.rs",
        "9181a69ce876c525463373a5dc58d288435006182063a26be5e9200bb6d3950c",
    ),
    (
        "crates/comfy_model/src/clip_tokenizer.rs",
        "24eb0bf943a25e9cbe5c2d182e7597d996c928c38e8a8429731624082d53715c",
    ),
    (
        "crates/comfy_model/src/formats.rs",
        "dc324d2355ebf6d68128e3047e448b7a6428065c9673d5537adb4a8164896724",
    ),
    (
        "crates/comfy_model/src/model_store.rs",
        "14b0e402258deeac17086235833a8e43c47c8be85ce074b2fa5d0e7120d4591c",
    ),
    (
        "crates/comfy_model/src/slices/native_diffusion.rs",
        "4859809749fc4e14908663bf1a9fd07dab705b13d06260dedda4f383ef21e680",
    ),
    (
        "crates/comfy_runtime/src/native_execution_controller.rs",
        "c5ef5148c1b8b3f8244e997cd07f3093b3b076555a01aa5005efed44ed420256",
    ),
    (
        "crates/comfy_test_support/src/native_diffusion_fixture.rs",
        "1e295e60f90c3e2d875c20c487b6a16f397127053ac3c476ea528e26695489d4",
    ),
];

fn verify_tokenizer_implementation_closure(
    workspace: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    for (path, expected) in TOKENIZER_IMPLEMENTATION_CLOSURE {
        let actual = format!("{:x}", Sha256::digest(fs::read(workspace.join(path))?));
        assert_eq!(actual, expected, "tokenizer implementation drifted: {path}");
    }
    Ok(())
}

fn sentencepiece() -> Result<SentencePieceTokenizer, Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("tokenizer.model");
    fs::write(&path, sentencepiece_model_bytes())?;
    let cancellation = CancellationToken::default();
    let mut index = ArtifactIndex::default();
    index.add_root(ArtifactRoot::canonical(
        "tokenizer",
        "tokenizers",
        directory.path(),
        ["model"],
    )?)?;
    index.refresh(&cancellation)?;
    let key = ArtifactKey::new("tokenizer", "tokenizer.model")?;
    let mut store = ModelStore::new(ParserLimits::default())?;
    let loaded = store.load(&index, &key, &cancellation)?;
    let vocabulary =
        store.verified_sentencepiece_vocabulary(&index, &loaded, &key, &cancellation)?;
    Ok(SentencePieceTokenizer::from_verified_vocabulary(
        vocabulary,
    )?)
}

fn sentencepiece_model_bytes() -> Vec<u8> {
    let mut entries: Vec<(String, f32, u64)> = vec![
        ("<unk>".to_owned(), -10.0, 2_u64),
        ("<s>".to_owned(), 0.0, 3),
        ("</s>".to_owned(), 0.0, 3),
    ];
    for index in 3..10 {
        entries.push((format!("<unused{index}>"), -100.0, 5));
    }
    entries.extend([
        ("▁hello".to_owned(), -1.0, 1),
        ("▁world".to_owned(), -1.0, 1),
        ("▁a".to_owned(), -1.0, 1),
        ("b".to_owned(), -1.0, 1),
        ("c".to_owned(), -1.0, 1),
        ("d".to_owned(), -1.0, 1),
        ("e".to_owned(), -1.0, 1),
        ("f".to_owned(), -1.0, 1),
        ("▁ab".to_owned(), 5.0, 1),
    ]);
    while entries.len() < 99 {
        let index = entries.len();
        entries.push((format!("<unused{index}>"), -100.0, 5));
    }
    entries.push(("<image>".to_owned(), 10.0, 4));

    let mut model = Vec::new();
    for (piece, score, piece_type) in entries {
        let mut encoded = Vec::new();
        encoded.push(0x0a);
        push_varint(&mut encoded, piece.len() as u64);
        encoded.extend_from_slice(piece.as_bytes());
        encoded.push(0x15);
        encoded.extend_from_slice(&score.to_le_bytes());
        encoded.push(0x18);
        push_varint(&mut encoded, piece_type);
        model.push(0x0a);
        push_varint(&mut model, encoded.len() as u64);
        model.extend(encoded);
    }
    model
}

fn push_varint(output: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        output.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn python_symbol_sha256(source: &[u8], symbol: &str) -> Result<String, Box<dyn std::error::Error>> {
    let source = std::str::from_utf8(source)?;
    let lines = source.split_inclusive('\n').collect::<Vec<_>>();
    let function_prefix = format!("def {symbol}(");
    let class_with_bases_prefix = format!("class {symbol}(");
    let class_without_bases_prefix = format!("class {symbol}:");
    let start = lines
        .iter()
        .position(|line| {
            line.starts_with(&function_prefix)
                || line.starts_with(&class_with_bases_prefix)
                || line.starts_with(&class_without_bases_prefix)
        })
        .ok_or_else(|| format!("Python symbol {symbol:?} is missing"))?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find_map(|(index, line)| {
            let trimmed = line.trim();
            (!trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !line.starts_with(char::is_whitespace))
            .then_some(index)
        })
        .unwrap_or(lines.len());
    let mut body_end = end;
    while body_end > start + 1 {
        let trimmed = lines[body_end - 1].trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            body_end -= 1;
        } else {
            break;
        }
    }
    Ok(format!(
        "{:x}",
        Sha256::digest(lines[start..body_end].concat().as_bytes())
    ))
}

fn configuration(maximum_length: usize) -> TokenizerConfiguration {
    TokenizerConfiguration {
        maximum_length,
        minimum_length: None,
        minimum_padding: None,
        pad_to_maximum_length: true,
        pad_left: false,
        start_token: Some(1),
        end_token: Some(2),
        pad_token: 2,
        maximum_word_length: 8,
        disable_weights: false,
        embedding_width: None,
    }
}

fn numeric_tokens(prompt: &comfy_model::NativeTokenizedPrompt) -> Vec<Vec<u32>> {
    prompt
        .sections()
        .iter()
        .map(|section| {
            section
                .tokens()
                .iter()
                .filter_map(|token| match token.value() {
                    NativeTokenValue::Token(value) => Some(*value),
                    NativeTokenValue::Embedding { .. } => None,
                })
                .collect()
        })
        .collect()
}

fn write_f32_safetensors(
    path: &Path,
    tensors: &[(&str, Vec<u64>, Vec<f32>)],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut header = serde_json::Map::new();
    let mut data = Vec::new();
    for (name, shape, values) in tensors {
        let start = data.len();
        data.try_reserve(values.len().saturating_mul(4))?;
        for value in values {
            data.extend_from_slice(&value.to_le_bytes());
        }
        header.insert(
            (*name).to_owned(),
            json!({
                "dtype": "F32",
                "shape": shape,
                "data_offsets": [start, data.len()],
            }),
        );
    }
    let mut header = serde_json::to_vec(&header)?;
    while !header.len().is_multiple_of(8) {
        header.push(b' ');
    }
    let mut file = Vec::new();
    file.try_reserve(
        8_usize
            .saturating_add(header.len())
            .saturating_add(data.len()),
    )?;
    file.extend_from_slice(&(header.len() as u64).to_le_bytes());
    file.extend_from_slice(&header);
    file.extend_from_slice(&data);
    fs::write(path, file)?;
    Ok(())
}

fn write_stored_embedding_zip(
    path: &Path,
    entries: &[(&str, &[u8])],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut output = Vec::new();
    let mut central = Vec::new();
    for (name, data) in entries {
        let offset = u32::try_from(output.len())?;
        let name_bytes = name.as_bytes();
        let crc = crc32(data);
        output.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
        output.extend_from_slice(&20_u16.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output.extend_from_slice(&crc.to_le_bytes());
        output.extend_from_slice(&u32::try_from(data.len())?.to_le_bytes());
        output.extend_from_slice(&u32::try_from(data.len())?.to_le_bytes());
        output.extend_from_slice(&u16::try_from(name_bytes.len())?.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output.extend_from_slice(name_bytes);
        output.extend_from_slice(data);

        central.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
        central.extend_from_slice(&0x0314_u16.to_le_bytes());
        central.extend_from_slice(&20_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&u32::try_from(data.len())?.to_le_bytes());
        central.extend_from_slice(&u32::try_from(data.len())?.to_le_bytes());
        central.extend_from_slice(&u16::try_from(name_bytes.len())?.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&(0o100644_u32 << 16).to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name_bytes);
    }
    let central_offset = u32::try_from(output.len())?;
    let central_size = u32::try_from(central.len())?;
    output.extend_from_slice(&central);
    output.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    output.extend_from_slice(&0_u16.to_le_bytes());
    output.extend_from_slice(&0_u16.to_le_bytes());
    output.extend_from_slice(&u16::try_from(entries.len())?.to_le_bytes());
    output.extend_from_slice(&u16::try_from(entries.len())?.to_le_bytes());
    output.extend_from_slice(&central_size.to_le_bytes());
    output.extend_from_slice(&central_offset.to_le_bytes());
    output.extend_from_slice(&0_u16.to_le_bytes());
    fs::write(path, output)?;
    Ok(())
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

fn nested_string_to_param_pickle(storage_key: &str, elements: u8) -> Vec<u8> {
    let mut bytes = vec![0x80, 0x02, b'}', b'('];
    push_pickle_binunicode(&mut bytes, "string_to_param");
    bytes.extend_from_slice(&[b'}', b'(']);
    push_pickle_binunicode(&mut bytes, "*");
    bytes.extend_from_slice(b"ctorch._utils\n_rebuild_tensor_v2\n((");
    push_pickle_binunicode(&mut bytes, "storage");
    bytes.extend_from_slice(b"ctorch\nFloatStorage\n");
    push_pickle_binunicode(&mut bytes, storage_key);
    push_pickle_binunicode(&mut bytes, "cpu");
    bytes.extend_from_slice(&[b'K', elements, b't', b'Q', b'K', 0]);
    bytes.extend_from_slice(&[b'(', b'K', elements, b't']);
    bytes.extend_from_slice(&[b'(', b'K', 1, b't']);
    bytes.extend_from_slice(&[0x89, b'}', b't', b'R', b'u', b'u', b'.']);
    bytes
}

fn push_pickle_binunicode(bytes: &mut Vec<u8>, value: &str) {
    bytes.push(b'X');
    bytes.extend_from_slice(&u32::try_from(value.len()).unwrap_or(u32::MAX).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn rust_sources_below(path: &Path) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let mut pending = vec![path.to_path_buf()];
    let mut sources = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("._"))
            {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                sources.push((path.display().to_string(), fs::read_to_string(path)?));
            }
        }
    }
    Ok(sources)
}

#[test]
fn weighting_matches_source_parentheses_explicit_values_and_escapes()
-> Result<(), Box<dyn std::error::Error>> {
    let parsed = parse_prompt_weights(
        r"plain (nested (deep):2.5) escaped \(literal\)",
        &CancellationToken::default(),
    )?;
    assert_eq!(
        parsed.iter().map(|item| item.text()).collect::<Vec<_>>(),
        ["plain ", "nested ", "deep", " escaped (literal)"]
    );
    let weights = parsed.iter().map(|item| item.weight()).collect::<Vec<_>>();
    assert!((weights[0] - 1.0).abs() < f32::EPSILON);
    assert!((weights[1] - 2.5).abs() < f32::EPSILON);
    assert!((weights[2] - 2.75).abs() < f32::EPSILON);
    assert!((weights[3] - 1.0).abs() < f32::EPSILON);
    Ok(())
}

#[test]
fn parser_is_bounded_and_cancellation_is_canonical() {
    let cancellation = CancellationToken::default();
    cancellation.cancel();
    assert!(matches!(
        parse_prompt_weights("hello", &cancellation),
        Err(NativeTokenizerError::Cancellation(_))
    ));
    assert!(matches!(
        parse_prompt_weights(
            &"x".repeat(comfy_model::MAX_NATIVE_PROMPT_BYTES + 1),
            &CancellationToken::default()
        ),
        Err(NativeTokenizerError::PromptTooLarge(_))
    ));
    let deep = format!("{}x{}", "(".repeat(130), ")".repeat(130));
    assert!(matches!(
        parse_prompt_weights(&deep, &CancellationToken::default()),
        Err(NativeTokenizerError::WeightNestingTooDeep(_))
    ));

    let mut unweighted_configuration = configuration(8);
    unweighted_configuration.disable_weights = true;
    let unweighted = NativePromptTokenizer::checked(
        NativeTokenizerFamily::SentencePiece(sentencepiece().expect("verified vocabulary")),
        unweighted_configuration,
        BTreeMap::new(),
    )
    .expect("valid unweighted tokenizer");
    assert!(matches!(
        unweighted.tokenize(
            &"x".repeat(comfy_model::MAX_NATIVE_PROMPT_BYTES + 1),
            &CancellationToken::default(),
        ),
        Err(NativeTokenizerError::PromptTooLarge(_))
    ));
}

#[test]
fn sentencepiece_special_tokens_decode_and_unknown_contract_are_exact()
-> Result<(), Box<dyn std::error::Error>> {
    let tokenizer = sentencepiece()?;
    let cancellation = CancellationToken::default();
    assert_eq!(
        tokenizer.decode(&[10, 11], false, &cancellation)?,
        "hello world"
    );
    assert_eq!(
        tokenizer.decode(&[10, 99, 11], true, &cancellation)?,
        "hello world"
    );
    assert_eq!(tokenizer.decode(&[10, 0], false, &cancellation)?, "hello�");
    assert!(matches!(
        tokenizer.decode(&[8_888], false, &cancellation),
        Err(NativeTokenizerError::UnknownToken(8_888))
    ));
    Ok(())
}

#[test]
fn sentencepiece_accepts_only_current_model_store_verified_vocabulary()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("tokenizer.model");
    fs::write(&path, sentencepiece_model_bytes())?;
    let cancellation = CancellationToken::default();
    let mut index = ArtifactIndex::default();
    index.add_root(ArtifactRoot::canonical(
        "tokenizer",
        "tokenizers",
        directory.path(),
        ["model"],
    )?)?;
    index.refresh(&cancellation)?;
    let key = ArtifactKey::new("tokenizer", "tokenizer.model")?;
    let mut store = ModelStore::new(ParserLimits::default())?;
    let loaded = store.load(&index, &key, &cancellation)?;
    let verified = store.verified_sentencepiece_vocabulary(&index, &loaded, &key, &cancellation)?;
    let tokenizer = SentencePieceTokenizer::from_verified_vocabulary(verified.clone())?;
    assert!(tokenizer.matches_verified_vocabulary(&verified));

    let mut foreign_store = ModelStore::new(ParserLimits::default())?;
    let foreign_loaded = foreign_store.load(&index, &key, &cancellation)?;
    let foreign_verified = foreign_store.verified_sentencepiece_vocabulary(
        &index,
        &foreign_loaded,
        &key,
        &cancellation,
    )?;
    assert!(!tokenizer.matches_verified_vocabulary(&foreign_verified));
    assert!(matches!(
        foreign_store.verified_sentencepiece_vocabulary(&index, &loaded, &key, &cancellation,),
        Err(comfy_model::ModelStoreError::ForeignModelHandle)
    ));

    let mut changed = sentencepiece_model_bytes();
    changed.push(0);
    fs::write(path, changed)?;
    assert!(
        store
            .verified_sentencepiece_vocabulary(&index, &loaded, &key, &cancellation)
            .is_err()
    );
    Ok(())
}

#[test]
fn sentencepiece_uses_canonical_scores_types_and_viterbi_segmentation()
-> Result<(), Box<dyn std::error::Error>> {
    let tokenizer = NativePromptTokenizer::checked(
        NativeTokenizerFamily::SentencePiece(sentencepiece()?),
        configuration(5),
        BTreeMap::new(),
    )?;
    let prompt = tokenizer.tokenize("ab", &CancellationToken::default())?;
    assert_eq!(numeric_tokens(&prompt), [vec![1, 18, 2, 2, 2]]);
    Ok(())
}

#[test]
fn multi_section_packing_never_silently_truncates_and_preserves_word_ids()
-> Result<(), Box<dyn std::error::Error>> {
    let tokenizer = NativePromptTokenizer::checked(
        NativeTokenizerFamily::SentencePiece(sentencepiece()?),
        configuration(4),
        BTreeMap::new(),
    )?;
    let prompt = tokenizer.tokenize(
        "hello (world) hello (world) hello",
        &CancellationToken::default(),
    )?;
    assert_eq!(prompt.sections().len(), 3);
    assert_eq!(
        numeric_tokens(&prompt),
        vec![vec![1, 10, 11, 2], vec![1, 10, 11, 2], vec![1, 10, 2, 2]]
    );
    let word_ids = prompt
        .sections()
        .iter()
        .flat_map(|section| section.tokens())
        .filter(|token| token.word_id() != 0)
        .map(|token| token.word_id())
        .collect::<Vec<_>>();
    assert_eq!(word_ids, [1, 2, 3, 4, 5]);
    Ok(())
}

#[test]
fn long_words_split_across_sections_with_one_stable_word_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let mut config = configuration(5);
    config.maximum_word_length = 2;
    let tokenizer = NativePromptTokenizer::checked(
        NativeTokenizerFamily::SentencePiece(sentencepiece()?),
        config,
        BTreeMap::new(),
    )?;
    let prompt = tokenizer.tokenize("abcdef", &CancellationToken::default())?;
    assert_eq!(prompt.sections().len(), 2);
    assert_eq!(
        numeric_tokens(&prompt),
        vec![vec![1, 18, 14, 15, 2], vec![1, 16, 17, 2, 2]]
    );
    assert!(
        prompt
            .sections()
            .iter()
            .flat_map(|section| section.tokens())
            .filter(|token| token.word_id() != 0)
            .all(|token| token.word_id() == 1)
    );
    Ok(())
}

#[test]
fn groups_larger_than_empty_section_capacity_make_bounded_progress()
-> Result<(), Box<dyn std::error::Error>> {
    let tokenizer = NativePromptTokenizer::checked(
        NativeTokenizerFamily::SentencePiece(sentencepiece()?),
        configuration(4),
        BTreeMap::new(),
    )?;
    let prompt = tokenizer.tokenize("abcdef", &CancellationToken::default())?;
    assert_eq!(
        numeric_tokens(&prompt),
        vec![vec![1, 18, 14, 2], vec![1, 15, 16, 2], vec![1, 17, 2, 2]]
    );
    assert!(
        prompt
            .sections()
            .iter()
            .flat_map(|section| section.tokens())
            .filter(|token| token.word_id() != 0)
            .all(|token| token.word_id() == 1)
    );
    Ok(())
}

#[test]
fn configurable_left_padding_minimums_and_list_batches_are_checked()
-> Result<(), Box<dyn std::error::Error>> {
    let mut config = configuration(8);
    config.pad_to_maximum_length = false;
    config.pad_left = true;
    config.pad_token = 7;
    config.minimum_padding = Some(2);
    config.minimum_length = Some(7);
    let tokenizer = NativePromptTokenizer::checked(
        NativeTokenizerFamily::SentencePiece(sentencepiece()?),
        config,
        BTreeMap::new(),
    )?;
    let prompt = tokenizer.tokenize("hello", &CancellationToken::default())?;
    assert_eq!(numeric_tokens(&prompt), vec![vec![7, 7, 7, 7, 1, 10, 2]]);
    let batch = tokenizer.tokenize_list(
        &["hello".to_owned(), "world".to_owned()],
        &CancellationToken::default(),
    )?;
    assert_eq!(batch.len(), 2);
    assert!(matches!(
        tokenizer.tokenize_list(&[], &CancellationToken::default()),
        Err(NativeTokenizerError::InvalidBatchSize(0))
    ));

    let mut overflow_config = configuration(5);
    overflow_config.pad_to_maximum_length = false;
    overflow_config.minimum_padding = Some(2);
    overflow_config.minimum_length = Some(7);
    overflow_config.maximum_word_length = 2;
    let overflow_tokenizer = NativePromptTokenizer::checked(
        NativeTokenizerFamily::SentencePiece(sentencepiece()?),
        overflow_config,
        BTreeMap::new(),
    )?;
    let overflow =
        overflow_tokenizer.tokenize("hello world hello world", &CancellationToken::default())?;
    assert_eq!(
        numeric_tokens(&overflow),
        vec![vec![1, 10, 11, 10, 2], vec![1, 11, 2, 2, 2, 2, 2]]
    );
    Ok(())
}

#[test]
fn textual_inversion_uses_only_a_canonical_verified_artifact_and_is_atomic()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("subject.safetensors");
    write_f32_safetensors(&path, &[("clip_l", vec![2, 2], vec![1.0, 2.0, 3.0, 4.0])])?;
    let mut index = ArtifactIndex::default();
    index.add_root(ArtifactRoot::canonical(
        "embeddings",
        "embeddings",
        directory.path(),
        ["safetensors"],
    )?)?;
    let cancellation = CancellationToken::default();
    index.refresh(&cancellation)?;
    let key = ArtifactKey::new("embeddings", "subject.safetensors")?;
    let mut store = ModelStore::new(ParserLimits::default())?;
    let model = store.load(&index, &key, &cancellation)?;
    let payload = store.verified_tensor_payload(&index, &model, &key, &cancellation)?;
    let embedding = TextualInversionEmbedding::from_verified_tensor_payload(
        payload,
        Some("clip_l"),
        2,
        &cancellation,
    )?;
    assert_eq!(embedding.rows().len(), 2);
    assert_eq!(&*embedding.rows()[1], &[3.0, 4.0]);
    let original_scope_embedding = embedding.clone();

    let mut embedding_configuration = configuration(6);
    embedding_configuration.embedding_width = Some(2);
    let tokenizer = NativePromptTokenizer::checked(
        NativeTokenizerFamily::SentencePiece(sentencepiece()?),
        embedding_configuration,
        BTreeMap::from([("subject".to_owned(), embedding)]),
    )?;
    let prompt = tokenizer.tokenize("embedding:subject, hello", &cancellation)?;
    let embeddings = prompt.sections()[0]
        .tokens()
        .iter()
        .filter_map(|token| match token.value() {
            NativeTokenValue::Embedding { values, .. } => Some(values.clone()),
            NativeTokenValue::Token(_) => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(embeddings.len(), 2);
    assert_eq!(&*embeddings[0], &[1.0, 2.0]);
    assert_eq!(prompt.sections()[0].tokens()[1].weight(), 1.0);
    assert_eq!(
        numeric_tokens(&prompt),
        vec![vec![1, 0, 0, 2], vec![1, 10, 2, 2, 2, 2]]
    );
    assert_eq!(
        numeric_tokens(&tokenizer.tokenize("embedding:missing,", &cancellation)?),
        vec![vec![1, 0, 0, 2, 2, 2]]
    );

    let payload = store.verified_tensor_payload(&index, &model, &key, &cancellation)?;
    assert!(matches!(
        TextualInversionEmbedding::from_verified_tensor_payload(
            payload,
            Some("clip_l"),
            3,
            &cancellation
        ),
        Err(NativeTokenizerError::InvalidEmbeddingShape { .. })
            | Err(NativeTokenizerError::EmbeddingWidthMismatch { .. })
    ));
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(matches!(
        store.verified_tensor_payload(&index, &model, &key, &cancelled),
        Err(comfy_model::ModelStoreError::Cancelled)
    ));

    write_f32_safetensors(&path, &[("clip_l", vec![1, 2], vec![7.0, 7.0])])?;
    assert!(
        store
            .verified_tensor_payload(&index, &model, &key, &cancellation)
            .is_err()
    );

    let foreign_directory = tempfile::tempdir()?;
    write_f32_safetensors(
        &foreign_directory.path().join("other.safetensors"),
        &[("clip_l", vec![1, 2], vec![9.0, 9.0])],
    )?;
    let mut foreign_index = ArtifactIndex::default();
    foreign_index.add_root(ArtifactRoot::canonical(
        "other",
        "embeddings",
        foreign_directory.path(),
        ["safetensors"],
    )?)?;
    foreign_index.refresh(&cancellation)?;
    let foreign_key = ArtifactKey::new("other", "other.safetensors")?;
    assert!(matches!(
        store.verified_tensor_payload(&foreign_index, &model, &foreign_key, &cancellation),
        Err(comfy_model::ModelStoreError::MissingTensorPayloadSource(_))
    ));
    let mut foreign_store = ModelStore::new(ParserLimits::default())?;
    let foreign_model = foreign_store.load(&foreign_index, &foreign_key, &cancellation)?;
    let foreign_payload = foreign_store.verified_tensor_payload(
        &foreign_index,
        &foreign_model,
        &foreign_key,
        &cancellation,
    )?;
    let foreign_embedding = TextualInversionEmbedding::from_verified_tensor_payload(
        foreign_payload,
        Some("clip_l"),
        2,
        &cancellation,
    )?;
    let mut mixed_configuration = configuration(6);
    mixed_configuration.embedding_width = Some(2);
    assert!(matches!(
        NativePromptTokenizer::checked(
            NativeTokenizerFamily::SentencePiece(sentencepiece()?),
            mixed_configuration,
            BTreeMap::from([
                ("subject".to_owned(), original_scope_embedding),
                ("foreign".to_owned(), foreign_embedding),
            ]),
        ),
        Err(NativeTokenizerError::ArtifactMismatch(_))
    ));
    Ok(())
}

#[test]
fn textual_inversion_selection_priority_and_bundle_order_match_source()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    write_f32_safetensors(
        &directory.path().join("wrapper.safetensors"),
        &[
            ("clip_l", vec![1, 2], vec![9.0, 9.0]),
            ("string_to_param.b", vec![1, 2], vec![2.0, 2.0]),
            ("string_to_param.a", vec![1, 2], vec![1.0, 1.0]),
        ],
    )?;
    write_f32_safetensors(
        &directory.path().join("exact.safetensors"),
        &[
            ("fallback", vec![1, 2], vec![8.0, 8.0]),
            ("clip_l", vec![1, 2], vec![3.0, 3.0]),
        ],
    )?;
    write_f32_safetensors(
        &directory.path().join("bundle.safetensors"),
        &[
            ("fallback", vec![1, 2], vec![8.0, 8.0]),
            ("bundle_emb.1.clip_l", vec![1, 2], vec![5.0, 5.0]),
            ("bundle_emb.0.clip_l", vec![1, 2], vec![4.0, 4.0]),
        ],
    )?;
    fs::write(
        directory.path().join("raw.bin"),
        [1.0_f32, 2.0_f32]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )?;
    let nested_pickle = nested_string_to_param_pickle("0", 2);
    let nested_values = [7.0_f32, 6.0]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect::<Vec<_>>();
    write_stored_embedding_zip(
        &directory.path().join("nested.ckpt"),
        &[
            ("archive/data.pkl", &nested_pickle),
            ("archive/data/0", &nested_values),
        ],
    )?;
    let cancellation = CancellationToken::default();
    let mut index = ArtifactIndex::default();
    index.add_root(ArtifactRoot::canonical(
        "embeddings",
        "embeddings",
        directory.path(),
        ["safetensors", "ckpt"],
    )?)?;
    index.refresh(&cancellation)?;
    let mut store = ModelStore::new(ParserLimits::default())?;

    let load = |store: &mut ModelStore, name: &str| {
        let key = ArtifactKey::new("embeddings", name)?;
        let model = store.load(&index, &key, &cancellation)?;
        let payload = store.verified_tensor_payload(&index, &model, &key, &cancellation)?;
        TextualInversionEmbedding::from_verified_tensor_payload(
            payload,
            Some("clip_l"),
            2,
            &cancellation,
        )
        .map_err(Box::<dyn std::error::Error>::from)
    };
    let wrapper = load(&mut store, "wrapper.safetensors")?;
    assert_eq!(&*wrapper.rows()[0], &[9.0, 9.0]);
    let exact = load(&mut store, "exact.safetensors")?;
    assert_eq!(&*exact.rows()[0], &[3.0, 3.0]);
    let bundle = load(&mut store, "bundle.safetensors")?;
    assert_eq!(bundle.rows().len(), 2);
    assert_eq!(&*bundle.rows()[0], &[5.0, 5.0]);
    assert_eq!(&*bundle.rows()[1], &[4.0, 4.0]);
    let nested = load(&mut store, "nested.ckpt")?;
    assert_eq!(&*nested.rows()[0], &[7.0, 6.0]);
    Ok(())
}

#[test]
fn canonical_sd1_bpe_owner_is_reused_for_source_fixture() -> Result<(), Box<dyn std::error::Error>>
{
    let vocabulary = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../comfy_test_support/fixtures/models/sd15-tiny-v1/vocab.json"
    ))?;
    let merges = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../comfy_test_support/fixtures/models/sd15-tiny-v1/merges.txt"
    ))?;
    let tokenizer = ClipBpeTokenizer::from_json_and_merges(
        ModelTokenizerDescriptor::checked("comfy.sd1.tokenizer")?,
        &vocabulary,
        &merges,
    )?;
    let family = NativeTokenizerFamily::ClipBpe(tokenizer);
    assert_eq!(
        family.decode(&[320, 1_628], false, &CancellationToken::default())?,
        "a test"
    );
    let prompt = NativePromptTokenizer::checked(family, configuration(6), BTreeMap::new())?
        .tokenize("a test", &CancellationToken::default())?;
    assert_eq!(numeric_tokens(&prompt), vec![vec![1, 320, 1_628, 2, 2, 2]]);

    let punctuation_prompt = NativePromptTokenizer::checked(
        NativeTokenizerFamily::ClipBpe(ClipBpeTokenizer::from_json_and_merges(
            ModelTokenizerDescriptor::checked("comfy.sd1.tokenizer")?,
            &vocabulary,
            &merges,
        )?),
        configuration(12),
        BTreeMap::new(),
    )?
    .tokenize("a  test, a", &CancellationToken::default())?;
    let content_word_ids = punctuation_prompt.sections()[0]
        .tokens()
        .iter()
        .filter(|token| token.word_id() != 0)
        .map(|token| token.word_id())
        .collect::<Vec<_>>();
    assert!(content_word_ids.len() >= 4);
    assert!(content_word_ids.iter().all(|word_id| *word_id == 1));
    Ok(())
}

#[test]
fn canonical_sd1_content_api_preserves_long_prompts_while_fixed_adapter_stays_77()
-> Result<(), Box<dyn std::error::Error>> {
    let vocabulary = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../comfy_test_support/fixtures/models/sd15-tiny-v1/vocab.json"
    ))?;
    let merges = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../comfy_test_support/fixtures/models/sd15-tiny-v1/merges.txt"
    ))?;
    let prompt = "a ".repeat(100);
    let canonical = comfy_model::clip::Sd1Tokenizer::from_json_and_merges(
        ModelTokenizerDescriptor::checked("comfy.sd1.tokenizer")?,
        &vocabulary,
        &merges,
    )?;
    assert_eq!(
        canonical
            .encode_content(&prompt, &CancellationToken::default())?
            .len(),
        100
    );
    let fixed = canonical.encode(&prompt, &CancellationToken::default())?;
    assert_eq!(fixed.tokens().len(), comfy_model::clip::SD1_CONTEXT_LENGTH);
    assert_eq!(
        fixed.content_tokens(),
        comfy_model::clip::SD1_CONTEXT_LENGTH - 2
    );

    let tokenizer = NativePromptTokenizer::checked(
        NativeTokenizerFamily::ClipBpe(ClipBpeTokenizer::from_json_and_merges(
            ModelTokenizerDescriptor::checked("comfy.sd1.tokenizer")?,
            &vocabulary,
            &merges,
        )?),
        configuration(10),
        BTreeMap::new(),
    )?;
    let sectioned = tokenizer.tokenize(&prompt, &CancellationToken::default())?;
    assert_eq!(sectioned.sections().len(), 13);
    assert_eq!(
        sectioned
            .sections()
            .iter()
            .flat_map(|section| section.tokens())
            .filter(|token| matches!(token.value(), NativeTokenValue::Token(320)))
            .count(),
        100
    );
    Ok(())
}

#[test]
fn canonical_sd1_artifact_admission_rejects_noncanonical_id_domains_and_merge_cardinality()
-> Result<(), Box<dyn std::error::Error>> {
    let vocabulary = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../comfy_test_support/fixtures/models/sd15-tiny-v1/vocab.json"
    ))?;
    let merges = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../comfy_test_support/fixtures/models/sd15-tiny-v1/merges.txt"
    ))?;
    let mut malformed_vocabulary = serde_json::from_str::<BTreeMap<String, u32>>(&vocabulary)?;
    let (_, token) = malformed_vocabulary
        .iter_mut()
        .find(|(_, token)| **token < comfy_model::clip::SD1_START_TOKEN)
        .ok_or("ordinary SD1 token is missing")?;
    *token = u32::try_from(comfy_model::clip::SD1_VOCABULARY_SIZE)?;
    let malformed_vocabulary = serde_json::to_string(&malformed_vocabulary)?;
    assert!(
        ClipBpeTokenizer::from_json_and_merges(
            ModelTokenizerDescriptor::checked("comfy.sd1.tokenizer")?,
            &malformed_vocabulary,
            &merges,
        )
        .is_err()
    );

    let mut truncated_merges = merges.lines().collect::<Vec<_>>();
    truncated_merges
        .pop()
        .ok_or("canonical merge tail is missing")?;
    let truncated_merges = truncated_merges.join("\n");
    assert!(
        ClipBpeTokenizer::from_json_and_merges(
            ModelTokenizerDescriptor::checked("comfy.sd1.tokenizer")?,
            &vocabulary,
            &truncated_merges,
        )
        .is_err()
    );

    let extended_merges = format!("{merges}__sim_invalid_left __sim_invalid_right\n");
    assert!(
        ClipBpeTokenizer::from_json_and_merges(
            ModelTokenizerDescriptor::checked("comfy.sd1.tokenizer")?,
            &vocabulary,
            &extended_merges,
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn full_weighted_segments_not_lexical_words_define_source_word_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let tokenizer = NativePromptTokenizer::checked(
        NativeTokenizerFamily::SentencePiece(sentencepiece()?),
        configuration(8),
        BTreeMap::new(),
    )?;
    let prompt = tokenizer.tokenize("hello,  world", &CancellationToken::default())?;
    let content_word_ids = prompt.sections()[0]
        .tokens()
        .iter()
        .filter(|token| token.word_id() != 0)
        .map(|token| token.word_id())
        .collect::<Vec<_>>();
    assert!(content_word_ids.len() >= 2);
    assert!(content_word_ids.iter().all(|word_id| *word_id == 1));
    let legacy_per_word_ids = (1..=content_word_ids.len() as u64).collect::<Vec<_>>();
    assert_ne!(content_word_ids, legacy_per_word_ids);
    Ok(())
}

#[test]
fn competing_foundational_owners_are_absent_from_the_adapter_source() {
    let source = include_str!("../src/clip_tokenizer.rs");
    for forbidden in [
        "struct CancellationToken",
        "struct ArtifactIndex",
        "struct ArtifactRoot",
        "struct ModelStore",
        "struct CpuBackend",
        "struct ExecutionContext",
        "struct Tensor",
        "canonicalize(",
    ] {
        assert!(!source.contains(forbidden), "duplicate owner: {forbidden}");
    }
    assert!(source.contains("Sd1Tokenizer::from_json_and_merges"));
    assert!(source.contains("VerifiedModelTensorPayload"));
    assert!(!source.contains("VerifiedArtifactFile"));
    assert!(source.contains("MAX_NATIVE_PROMPT_BYTES: usize = crate::clip::SD1_MAX_PROMPT_BYTES"));
    assert!(
        source
            .contains("MAX_NATIVE_WEIGHT_SEGMENTS: usize = crate::clip::SD1_MAX_WEIGHTED_SEGMENTS")
    );
    assert!(!source.contains("merge_ranks:"));
    assert!(!source.contains("byte_decoder:"));
    assert_eq!(source.matches("fn pack(").count(), 1);
    assert!(source.contains("pub struct ClipBpeTokenizer {\n    tokenizer: Sd1Tokenizer,\n}"));
    assert!(!source.contains("UnverifiedEmbeddingTensorRows"));
    assert!(!source.contains("project_verified_embedding_candidates"));
    assert!(!source.contains("concatenate_unverified_bundled_embedding_rows"));
    assert!(!source.contains("select_unverified_named_embedding_rows"));
    assert!(source.contains("VerifiedEmbeddingArchivePayload"));
    assert!(source.contains("VerifiedSentencePieceVocabulary"));
    assert_eq!(
        source
            .matches("pub fn from_verified_tensor_payload(")
            .count(),
        1
    );
    assert!(!source.contains("pub fn from_checked_rows("));
}

fn validate_fallible_allocation_atomicity_and_workspace_absence()
-> Result<(), Box<dyn std::error::Error>> {
    let source = include_str!("../src/clip_tokenizer.rs");
    assert!(source.contains("reserve_tokenizer_values(&mut sections"));
    assert!(source.contains(".try_reserve("));
    assert!(!source.contains(".reserve("));
    for forbidden in [
        "BackendWorkspaceAuthority",
        "CpuWorkspace",
        "ExecutionContext",
        "WorkspaceLease",
    ] {
        assert!(
            !source.contains(forbidden),
            "tokenizer unexpectedly owns canonical workspace state: {forbidden}"
        );
    }

    let mut config = configuration(3);
    config.maximum_word_length = 1;
    config.pad_to_maximum_length = false;
    let tokenizer = NativePromptTokenizer::checked(
        NativeTokenizerFamily::SentencePiece(sentencepiece()?),
        config,
        BTreeMap::new(),
    )?;
    let oversized = "hello ".repeat(comfy_model::MAX_NATIVE_TOKEN_SECTIONS + 1);
    assert!(matches!(
        tokenizer.tokenize(&oversized, &CancellationToken::default()),
        Err(NativeTokenizerError::TooManySections(_))
    ));
    assert_eq!(
        numeric_tokens(&tokenizer.tokenize("hello", &CancellationToken::default())?),
        [vec![1, 10, 2]]
    );
    Ok(())
}

#[test]
fn fallible_allocation_is_atomic_and_tokenization_owns_no_backend_workspace()
-> Result<(), Box<dyn std::error::Error>> {
    validate_fallible_allocation_atomicity_and_workspace_absence()
}

#[test]
fn production_call_scan_finds_no_embedding_or_tokenizer_bypass()
-> Result<(), Box<dyn std::error::Error>> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace root is unavailable")?;
    for (path, source) in rust_sources_below(&workspace.join("crates"))? {
        if !path.contains("/src/") || path.ends_with("crates/comfy_model/src/clip_tokenizer.rs") {
            continue;
        }
        for forbidden in [
            "TextualInversionEmbedding::from_verified_rows",
            "TextualInversionEmbedding::from_verified_raw_f32",
            "project_safe_embedding_zip_entries(",
            "SentencePieceTokenizer::from_vocabulary",
            "UnverifiedEmbeddingTensorRows",
            "project_verified_embedding_candidates",
            "concatenate_unverified_bundled_embedding_rows",
            "select_unverified_named_embedding_rows",
            "EmbeddingArchiveEntry",
        ] {
            assert!(
                !source.contains(forbidden),
                "production tokenizer bypass {forbidden:?} remains in {path}"
            );
        }
    }
    Ok(())
}

#[test]
fn production_call_scan_ignores_apple_double_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    fs::write(directory.path().join("source.rs"), "fn source() {}\n")?;
    fs::write(directory.path().join("._source.rs"), [0xff])?;
    assert_eq!(
        rust_sources_below(directory.path())?,
        [(
            directory.path().join("source.rs").display().to_string(),
            "fn source() {}\n".to_owned(),
        )]
    );
    Ok(())
}

#[test]
fn empty_token_and_baseline_weight_projections_match_source_equations()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        generate_empty_tokens(Some(1), Some(2), 7, 5)?,
        [1, 2, 7, 7, 7]
    );
    assert!(matches!(
        generate_empty_tokens(Some(1), Some(2), 7, 1),
        Err(NativeTokenizerError::InvalidEmptyTokenLength(1))
    ));
    let weighted = apply_empty_baseline_token_weights(
        &[vec![3.0, 5.0, 10.0, 14.0]],
        &[1.0, 1.0, 2.0, 2.0],
        &[vec![0.5, 2.0]],
        2,
    )?;
    assert_eq!(weighted, [vec![2.0, 3.0, 18.0, 26.0]]);
    assert!(matches!(
        apply_empty_baseline_token_weights(&[vec![1.0]], &[1.0], &[vec![1.0]], 2),
        Err(NativeTokenizerError::InvalidWeightProjection)
    ));
    Ok(())
}

#[test]
fn canonical_archive_record_bundle_and_named_projections_are_checked()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let values = (0..768)
        .flat_map(|index| (index as f32).to_le_bytes())
        .collect::<Vec<_>>();
    write_stored_embedding_zip(
        &directory.path().join("archive.ckpt"),
        &[("archive/meta", b"bad"), ("archive/data/0", &values)],
    )?;
    write_stored_embedding_zip(
        &directory.path().join("ignored.ckpt"),
        &[("archive/data/0", b"bad")],
    )?;
    write_f32_safetensors(
        &directory.path().join("bundle.safetensors"),
        &[
            ("bundle_emb.1.clip_l", vec![1, 2], vec![1.0, 1.0]),
            ("bundle_emb.0.clip_l", vec![1, 2], vec![0.0, 0.0]),
            ("named", vec![1, 2], vec![2.0, 2.0]),
        ],
    )?;
    let cancellation = CancellationToken::default();
    let mut index = ArtifactIndex::default();
    index.add_root(ArtifactRoot::canonical(
        "embeddings",
        "embeddings",
        directory.path(),
        ["bin", "ckpt", "safetensors"],
    )?)?;
    index.refresh(&cancellation)?;
    let mut store = ModelStore::new(ParserLimits::default())?;
    assert!(
        store
            .load(
                &index,
                &ArtifactKey::new("embeddings", "raw.bin")?,
                &cancellation,
            )
            .is_err()
    );
    let archive_key = ArtifactKey::new("embeddings", "archive.ckpt")?;
    let projected = store
        .verified_embedding_archive(&index, &archive_key, &cancellation)?
        .ok_or("missing projected ZIP embedding")?;
    assert_eq!(projected.rows().len(), 1);
    assert_eq!(projected.width(), 768);
    let embedding =
        TextualInversionEmbedding::from_verified_archive_payload(projected, &cancellation)?;
    assert_eq!(embedding.rows().len(), 1);
    assert_eq!(embedding.artifact_key(), &archive_key);
    assert!(
        store
            .verified_embedding_archive(
                &index,
                &ArtifactKey::new("embeddings", "ignored.ckpt")?,
                &cancellation,
            )?
            .is_none()
    );
    let bundle_key = ArtifactKey::new("embeddings", "bundle.safetensors")?;
    let model = store.load(&index, &bundle_key, &cancellation)?;
    let payload = store.verified_tensor_payload(&index, &model, &bundle_key, &cancellation)?;
    let bundled = TextualInversionEmbedding::from_verified_tensor_payload(
        payload,
        Some("clip_l"),
        2,
        &cancellation,
    )?;
    assert_eq!(bundled.rows().len(), 2);
    assert_eq!(&*bundled.rows()[0], &[1.0, 1.0]);
    assert_eq!(&*bundled.rows()[1], &[0.0, 0.0]);
    Ok(())
}

#[test]
fn unmatched_parentheses_and_escape_sequences_match_source_edge_behavior()
-> Result<(), Box<dyn std::error::Error>> {
    let unmatched_close = parse_prompt_weights("a)b", &CancellationToken::default())?;
    assert_eq!(unmatched_close.len(), 1);
    assert_eq!(unmatched_close[0].text(), "a)b");
    let unmatched_open = parse_prompt_weights("a(b", &CancellationToken::default())?;
    assert_eq!(unmatched_open.len(), 2);
    assert_eq!(unmatched_open[1].text(), "(b");
    let escaped = parse_prompt_weights(r"\(a:9\)", &CancellationToken::default())?;
    assert_eq!(escaped[0].text(), "(a:9)");
    assert_eq!(escaped[0].weight(), 1.0);
    Ok(())
}

#[test]
fn source_row_closure_is_exact_unique_and_backed_by_valid_and_invalid_fixtures()
-> Result<(), Box<dyn std::error::Error>> {
    let expected = [
        "gen_empty_tokens",
        "ClipTokenWeightEncoder",
        "parse_parentheses",
        "token_weights",
        "escape_important",
        "unescape_important",
        "safe_load_embed_zip",
        "expand_directory_list",
        "bundled_embed",
        "load_embed",
        "SDTokenizer",
        "SD1Tokenizer",
    ];
    assert_eq!(CLIP_TOKENIZER_SOURCE_ROWS, expected);
    let unique = CLIP_TOKENIZER_SOURCE_ROWS
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), 12);
    let test_source = include_str!("clip_tokenizer.rs");
    for fixture in [
        "empty_token_and_baseline_weight_projections_match_source_equations",
        "weighting_matches_source_parentheses_explicit_values_and_escapes",
        "unmatched_parentheses_and_escape_sequences_match_source_edge_behavior",
        "canonical_archive_record_bundle_and_named_projections_are_checked",
        "multi_section_packing_never_silently_truncates_and_preserves_word_ids",
        "canonical_sd1_bpe_owner_is_reused_for_source_fixture",
    ] {
        assert!(test_source.contains(fixture));
    }
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace root is unavailable")?;
    let catalog = fs::read_to_string(
        workspace.join(".agents/specs/comfy-parity/catalogs/backend-conditioning-contracts.csv"),
    )?;
    verify_tokenizer_implementation_closure(workspace)?;
    validate_fallible_allocation_atomicity_and_workspace_absence()?;
    let source_path = "projects/comfy/ComfyUI/comfy/sd1_clip.py";
    let source = fs::read(workspace.join(source_path))?;
    let mut matched = Vec::new();
    for line in catalog.lines().skip(1) {
        let fields = line.split(',').collect::<Vec<_>>();
        if fields.get(8) != Some(&"comfy-parity-clip-tokenizer-foundation") {
            continue;
        }
        assert_eq!(fields[2], source_path);
        assert_eq!(python_symbol_sha256(&source, fields[3])?, fields[6]);
        execute_tokenizer_catalog_contract(fields[3])?;
        execute_invalid_tokenizer_catalog_contract(fields[3])?;
        matched.push(fields[3]);
    }
    assert_eq!(matched, CLIP_TOKENIZER_SOURCE_ROWS);
    Ok(())
}

fn execute_tokenizer_catalog_contract(symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
    let cancellation = CancellationToken::default();
    match symbol {
        "gen_empty_tokens" => {
            assert_eq!(
                generate_empty_tokens(Some(1), Some(2), 7, 5)?,
                [1, 2, 7, 7, 7]
            );
        }
        "ClipTokenWeightEncoder" => {
            assert_eq!(
                apply_empty_baseline_token_weights(
                    &[vec![3.0, 5.0, 10.0, 14.0]],
                    &[1.0, 1.0, 2.0, 2.0],
                    &[vec![0.5, 2.0]],
                    2,
                )?,
                [vec![2.0, 3.0, 18.0, 26.0]]
            );
        }
        "parse_parentheses" => {
            assert_eq!(
                parse_parentheses("plain (nested (value)) tail", &cancellation)?,
                ["plain ", "(nested (value))", " tail"]
            );
        }
        "token_weights" => {
            let weighted = token_weights("plain (weighted:2.0)", 0.5, &cancellation)?;
            assert_eq!(weighted.len(), 2);
            assert_eq!(weighted[0].weight(), 0.5);
            assert_eq!(weighted[1].text(), "weighted");
            assert_eq!(weighted[1].weight(), 2.0);
        }
        "escape_important" => {
            assert_eq!(
                escape_important(r"literal \(value\)", &cancellation)?,
                "literal \0\u{2}value\0\u{1}"
            );
        }
        "unescape_important" => {
            assert_eq!(
                unescape_important("literal \0\u{2}value\0\u{1}", &cancellation)?,
                "literal (value)"
            );
        }
        "safe_load_embed_zip" => {
            let directory = tempfile::tempdir()?;
            let values = (0..768)
                .flat_map(|index| (index as f32).to_le_bytes())
                .collect::<Vec<_>>();
            write_stored_embedding_zip(
                &directory.path().join("subject.ckpt"),
                &[("archive/data/0", &values)],
            )?;
            let mut index = ArtifactIndex::default();
            index.add_root(ArtifactRoot::canonical(
                "embeddings",
                "embeddings",
                directory.path(),
                ["ckpt"],
            )?)?;
            index.refresh(&cancellation)?;
            let key = ArtifactKey::new("embeddings", "subject.ckpt")?;
            let store = ModelStore::new(ParserLimits::default())?;
            let rows = store
                .verified_embedding_archive(&index, &key, &cancellation)?
                .ok_or("safe embedding archive projection is missing")?;
            assert_eq!(rows.rows().len(), 1);
            assert_eq!(rows.width(), 768);
        }
        "expand_directory_list" => {
            let directory = tempfile::tempdir()?;
            let nested = directory.path().join("nested");
            fs::create_dir(&nested)?;
            fs::write(nested.join("subject.safetensors"), b"indexed embedding")?;
            let mut index = ArtifactIndex::default();
            index.add_root(ArtifactRoot::canonical(
                "embeddings",
                "embeddings",
                directory.path(),
                ["safetensors"],
            )?)?;
            index.refresh(&cancellation)?;
            assert_eq!(
                index
                    .records()
                    .map(|record| record.key.clone())
                    .collect::<Vec<_>>(),
                [ArtifactKey::new(
                    "embeddings",
                    "nested/subject.safetensors"
                )?]
            );
        }
        "bundled_embed" => {
            let directory = tempfile::tempdir()?;
            write_f32_safetensors(
                &directory.path().join("bundle.safetensors"),
                &[("bundle_emb.0.clip_l", vec![1, 2], vec![1.0, 2.0])],
            )?;
            let mut index = ArtifactIndex::default();
            index.add_root(ArtifactRoot::canonical(
                "embeddings",
                "embeddings",
                directory.path(),
                ["safetensors"],
            )?)?;
            index.refresh(&cancellation)?;
            let key = ArtifactKey::new("embeddings", "bundle.safetensors")?;
            let mut store = ModelStore::new(ParserLimits::default())?;
            let model = store.load(&index, &key, &cancellation)?;
            let payload = store.verified_tensor_payload(&index, &model, &key, &cancellation)?;
            assert_eq!(
                TextualInversionEmbedding::from_verified_tensor_payload(
                    payload,
                    Some("clip_l"),
                    2,
                    &cancellation,
                )?
                .rows()
                .len(),
                1
            );
        }
        "load_embed" => {
            let directory = tempfile::tempdir()?;
            let path = directory.path().join("subject.safetensors");
            write_f32_safetensors(&path, &[("clip_l", vec![1, 2], vec![1.0, 2.0])])?;
            let mut index = ArtifactIndex::default();
            index.add_root(ArtifactRoot::canonical(
                "embeddings",
                "embeddings",
                directory.path(),
                ["safetensors"],
            )?)?;
            index.refresh(&cancellation)?;
            let key = ArtifactKey::new("embeddings", "subject.safetensors")?;
            let mut store = ModelStore::new(ParserLimits::default())?;
            let model = store.load(&index, &key, &cancellation)?;
            let payload = store.verified_tensor_payload(&index, &model, &key, &cancellation)?;
            let embedding = TextualInversionEmbedding::from_verified_tensor_payload(
                payload,
                Some("clip_l"),
                2,
                &cancellation,
            )?;
            assert_eq!(embedding.rows().len(), 1);
        }
        "SDTokenizer" => {
            let directory = tempfile::tempdir()?;
            let path = directory.path().join("subject.safetensors");
            write_f32_safetensors(&path, &[("clip_l", vec![2, 2], vec![1.0, 2.0, 3.0, 4.0])])?;
            let mut index = ArtifactIndex::default();
            index.add_root(ArtifactRoot::canonical(
                "embeddings",
                "embeddings",
                directory.path(),
                ["safetensors"],
            )?)?;
            index.refresh(&cancellation)?;
            let key = ArtifactKey::new("embeddings", "subject.safetensors")?;
            let mut store = ModelStore::new(ParserLimits::default())?;
            let model = store.load(&index, &key, &cancellation)?;
            let payload = store.verified_tensor_payload(&index, &model, &key, &cancellation)?;
            let embedding = TextualInversionEmbedding::from_verified_tensor_payload(
                payload,
                Some("clip_l"),
                2,
                &cancellation,
            )?;
            let mut tokenizer_configuration = configuration(6);
            tokenizer_configuration.embedding_width = Some(2);
            let tokenizer = NativePromptTokenizer::checked(
                NativeTokenizerFamily::SentencePiece(sentencepiece()?),
                tokenizer_configuration,
                BTreeMap::from([("subject".to_owned(), embedding)]),
            )?;
            let tokenized = tokenizer.tokenize("embedding:subject, hello", &cancellation)?;
            assert_eq!(
                numeric_tokens(&tokenized),
                [vec![1, 0, 0, 2], vec![1, 10, 2, 2, 2, 2]]
            );
            assert_eq!(
                tokenized
                    .sections()
                    .iter()
                    .flat_map(|section| section.tokens())
                    .filter(|token| matches!(token.value(), NativeTokenValue::Embedding { .. }))
                    .count(),
                2
            );
            let mut overflow_configuration = configuration(5);
            overflow_configuration.pad_to_maximum_length = false;
            overflow_configuration.minimum_padding = Some(2);
            overflow_configuration.minimum_length = Some(7);
            overflow_configuration.maximum_word_length = 2;
            let overflow_tokenizer = NativePromptTokenizer::checked(
                NativeTokenizerFamily::SentencePiece(sentencepiece()?),
                overflow_configuration,
                BTreeMap::new(),
            )?;
            assert_eq!(
                numeric_tokens(
                    &overflow_tokenizer.tokenize("hello world hello world", &cancellation,)?
                ),
                [vec![1, 10, 11, 10, 2], vec![1, 11, 2, 2, 2, 2, 2]]
            );
        }
        "SD1Tokenizer" => {
            let vocabulary = fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../comfy_test_support/fixtures/models/sd15-tiny-v1/vocab.json"
            ))?;
            let merges = fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../comfy_test_support/fixtures/models/sd15-tiny-v1/merges.txt"
            ))?;
            let tokenizer = ClipBpeTokenizer::from_json_and_merges(
                ModelTokenizerDescriptor::checked("comfy.sd1.tokenizer")?,
                &vocabulary,
                &merges,
            )?;
            assert_eq!(
                tokenizer.decode(&[320, 1_628], false, &cancellation)?,
                "a test"
            );
        }
        unexpected => {
            return Err(format!("unaccounted tokenizer catalog symbol {unexpected}").into());
        }
    }
    Ok(())
}

fn execute_invalid_tokenizer_catalog_contract(
    symbol: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cancellation = CancellationToken::default();
    match symbol {
        "gen_empty_tokens" => assert!(matches!(
            generate_empty_tokens(Some(1), Some(2), 0, 1),
            Err(NativeTokenizerError::InvalidEmptyTokenLength(1))
        )),
        "ClipTokenWeightEncoder" => assert!(matches!(
            apply_empty_baseline_token_weights(&[vec![1.0]], &[1.0], &[vec![1.0, 1.0]], 1,),
            Err(NativeTokenizerError::InvalidWeightProjection)
        )),
        "parse_parentheses" => {
            let cancelled = CancellationToken::default();
            cancelled.cancel();
            assert!(matches!(
                parse_parentheses("(cancelled)", &cancelled),
                Err(NativeTokenizerError::Cancellation(_))
            ));
        }
        "token_weights" => assert!(matches!(
            token_weights("weighted", f32::NAN, &cancellation),
            Err(NativeTokenizerError::InvalidWeight(value)) if value.is_nan()
        )),
        "escape_important" => assert!(matches!(
            escape_important(
                &"x".repeat(comfy_model::MAX_NATIVE_PROMPT_BYTES + 1),
                &cancellation,
            ),
            Err(NativeTokenizerError::PromptTooLarge(_))
        )),
        "unescape_important" => {
            let cancelled = CancellationToken::default();
            cancelled.cancel();
            assert!(matches!(
                unescape_important("\0\u{2}", &cancelled),
                Err(NativeTokenizerError::Cancellation(_))
            ));
        }
        "safe_load_embed_zip" => {
            let directory = tempfile::tempdir()?;
            let mut values = vec![0_u8; 768 * 4];
            values[..4].copy_from_slice(&f32::NAN.to_le_bytes());
            write_stored_embedding_zip(
                &directory.path().join("invalid.ckpt"),
                &[("archive/data/0", &values)],
            )?;
            let mut index = ArtifactIndex::default();
            index.add_root(ArtifactRoot::canonical(
                "embeddings",
                "embeddings",
                directory.path(),
                ["ckpt"],
            )?)?;
            index.refresh(&cancellation)?;
            assert!(
                ModelStore::new(ParserLimits::default())?
                    .verified_embedding_archive(
                        &index,
                        &ArtifactKey::new("embeddings", "invalid.ckpt")?,
                        &cancellation,
                    )
                    .is_err()
            );
        }
        "expand_directory_list" => {
            assert!(matches!(
                ArtifactRoot::canonical("", "embeddings", Path::new("."), ["safetensors"]),
                Err(_)
            ));
        }
        "bundled_embed" => {
            let directory = tempfile::tempdir()?;
            write_f32_safetensors(
                &directory.path().join("invalid-bundle.safetensors"),
                &[("bundle_emb.0.clip_l", vec![1, 2], vec![1.0, 2.0])],
            )?;
            let mut index = ArtifactIndex::default();
            index.add_root(ArtifactRoot::canonical(
                "embeddings",
                "embeddings",
                directory.path(),
                ["safetensors"],
            )?)?;
            index.refresh(&cancellation)?;
            let key = ArtifactKey::new("embeddings", "invalid-bundle.safetensors")?;
            let mut store = ModelStore::new(ParserLimits::default())?;
            let model = store.load(&index, &key, &cancellation)?;
            let payload = store.verified_tensor_payload(&index, &model, &key, &cancellation)?;
            assert!(matches!(
                TextualInversionEmbedding::from_verified_tensor_payload(
                    payload,
                    Some("clip_l"),
                    3,
                    &cancellation,
                ),
                Err(NativeTokenizerError::EmbeddingWidthMismatch { expected: 3 })
            ));
        }
        "load_embed" => {
            let directory = tempfile::tempdir()?;
            let path = directory.path().join("wrong-width.safetensors");
            write_f32_safetensors(&path, &[("clip_l", vec![1, 2], vec![1.0, 2.0])])?;
            let mut index = ArtifactIndex::default();
            index.add_root(ArtifactRoot::canonical(
                "embeddings",
                "embeddings",
                directory.path(),
                ["safetensors"],
            )?)?;
            index.refresh(&cancellation)?;
            let key = ArtifactKey::new("embeddings", "wrong-width.safetensors")?;
            let mut store = ModelStore::new(ParserLimits::default())?;
            let model = store.load(&index, &key, &cancellation)?;
            let payload = store.verified_tensor_payload(&index, &model, &key, &cancellation)?;
            assert!(matches!(
                TextualInversionEmbedding::from_verified_tensor_payload(
                    payload,
                    Some("clip_l"),
                    3,
                    &cancellation,
                ),
                Err(NativeTokenizerError::InvalidEmbeddingShape { .. })
                    | Err(NativeTokenizerError::EmbeddingWidthMismatch { expected: 3 })
            ));
        }
        "SDTokenizer" => {
            let cancelled = CancellationToken::default();
            cancelled.cancel();
            let tokenizer = NativePromptTokenizer::checked(
                NativeTokenizerFamily::SentencePiece(sentencepiece()?),
                configuration(6),
                BTreeMap::new(),
            )?;
            assert!(matches!(
                tokenizer.tokenize("hello", &cancelled),
                Err(NativeTokenizerError::Cancellation(_))
            ));
        }
        "SD1Tokenizer" => {
            let vocabulary = fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../comfy_test_support/fixtures/models/sd15-tiny-v1/vocab.json"
            ))?;
            let merges = fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../comfy_test_support/fixtures/models/sd15-tiny-v1/merges.txt"
            ))?;
            let tokenizer = ClipBpeTokenizer::from_json_and_merges(
                ModelTokenizerDescriptor::checked("comfy.sd1.tokenizer")?,
                &vocabulary,
                &merges,
            )?;
            assert!(matches!(
                tokenizer.decode(&[u32::MAX], false, &cancellation),
                Err(NativeTokenizerError::Clip(_))
            ));
        }
        unexpected => return Err(format!("unaccounted tokenizer symbol {unexpected}").into()),
    }
    Ok(())
}

#[test]
fn val_clip_001_tokenizer_rows_execute_and_publish_partial_ledger()
-> Result<(), Box<dyn std::error::Error>> {
    const CONTRACT_TASK: &str = "comfy-parity-clip-tokenizer-foundation";
    const OWNER_RESULT_TASK: &str = "comfy-parity-sd1-tokenizer-owner-consolidation";
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace root is unavailable")?;
    let catalog = fs::read_to_string(
        workspace.join(".agents/specs/comfy-parity/catalogs/backend-conditioning-contracts.csv"),
    )?;
    verify_tokenizer_implementation_closure(workspace)?;
    validate_fallible_allocation_atomicity_and_workspace_absence()?;
    let mut contracts = Vec::new();
    let mut symbols = Vec::new();
    for line in catalog.lines().skip(1) {
        let fields = line.split(',').collect::<Vec<_>>();
        if fields.get(8).copied() != Some(CONTRACT_TASK) {
            continue;
        }
        assert_eq!(fields.len(), 15);
        assert_eq!(fields[7], "comfy_model::clip");
        assert_eq!(fields[9], "VAL-CLIP-001");
        assert_eq!(fields[10], "native_rust");
        assert_eq!(fields[14], "VAL-CLIP-001");
        let source = fs::read(workspace.join(fields[2]))?;
        assert_eq!(format!("{:x}", Sha256::digest(source)), fields[5]);
        let source = fs::read(workspace.join(fields[2]))?;
        assert_eq!(python_symbol_sha256(&source, fields[3])?, fields[6]);
        execute_tokenizer_catalog_contract(fields[3])?;
        execute_invalid_tokenizer_catalog_contract(fields[3])?;
        symbols.push(fields[3]);
        contracts.push(json!({
            "contract_id": fields[0],
            "task_id": CONTRACT_TASK,
            "source_sha256": fields[5],
            "symbol_sha256": fields[6],
            "status": "passed",
            "case_ids": [
                format!("{}:native-tokenizer-valid", fields[0]),
                format!("{}:native-tokenizer-invalid", fields[0]),
            ],
        }));
    }
    assert_eq!(symbols, CLIP_TOKENIZER_SOURCE_ROWS);
    assert_eq!(contracts.len(), CLIP_TOKENIZER_SOURCE_ROWS.len());

    let producer_path = "crates/comfy_model/tests/clip_tokenizer.rs";
    let producer_sha256 = format!(
        "{:x}",
        Sha256::digest(fs::read(workspace.join(producer_path))?)
    );
    let task_implementations = TOKENIZER_IMPLEMENTATION_CLOSURE
        .iter()
        .map(|(path, _)| {
            Ok(json!({
                "path": path,
                "sha256": format!("{:x}", Sha256::digest(fs::read(workspace.join(path))?)),
            }))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let tokenizer_task_implementations = task_implementations
        .iter()
        .filter(|implementation| {
            implementation
                .get("path")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|path| {
                    matches!(
                        path,
                        "crates/comfy_model/src/clip.rs"
                            | "crates/comfy_model/src/clip_tokenizer.rs"
                            | "crates/comfy_model/src/formats.rs"
                            | "crates/comfy_model/src/model_store.rs"
                    )
                })
        })
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(tokenizer_task_implementations.len(), 4);
    let task_results = BTreeMap::from([
        (
            OWNER_RESULT_TASK,
            json!({
                "status": "passed",
                "passed": contracts.len(),
                "failed": 0,
                "skipped": 0,
                "case_ids": [
                    "task512:resource-exhaustion-no-publication",
                    "task512:canonical-workspace-free",
                ],
                "implementations": task_implementations,
            }),
        ),
        (
            CONTRACT_TASK,
            json!({
                "status": "passed",
                "passed": contracts.len(),
                "failed": 0,
                "skipped": 0,
                "case_ids": [
                    "task348:resource-exhaustion-no-publication",
                    "task348:canonical-workspace-free",
                ],
                "implementations": tokenizer_task_implementations,
            }),
        ),
    ]);
    let artifact_directory = workspace.join("target/comfy-parity");
    let artifact_path = artifact_directory.join("val-clip-001.json");
    let previous_artifact = if artifact_path.exists() {
        Some(serde_json::from_slice::<Value>(&fs::read(&artifact_path)?)?)
    } else {
        None
    };
    let mut artifact = json!({
        "schema_version": 1,
        "validation_id": "VAL-CLIP-001",
        "overall_status": "partial",
        "environment": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "backend": "comfy_tensor::CpuBackend",
            "device": "cpu",
            "dtype": "f32",
        },
        "summary": {
            "passed": contracts.len() * 2,
            "failed": 0,
            "skipped": 0,
        },
        "implementation": {
            "path": producer_path,
            "sha256": producer_sha256,
        },
        "task_results": task_results,
        "contracts": contracts,
        "remaining_tasks": [
            "comfy-parity-clip-execution-foundation",
            "comfy-parity-clip-text-transformer-foundation",
            "comfy-parity-clip-vision-foundation",
            "comfy-parity-clip-text-encoder-breadth",
            "comfy-parity-clip-owner-consolidation"
        ],
    });
    if let Some(previous_artifact) = previous_artifact {
        let current_results = artifact
            .get_mut("task_results")
            .and_then(Value::as_object_mut)
            .ok_or("VAL-CLIP-001 task results are missing")?;
        if let Some(previous_results) = previous_artifact
            .get("task_results")
            .and_then(Value::as_object)
        {
            for (task, result) in previous_results {
                if task != CONTRACT_TASK && task != OWNER_RESULT_TASK {
                    current_results.insert(task.clone(), result.clone());
                }
            }
        }
        let current_contracts = artifact
            .get_mut("contracts")
            .and_then(Value::as_array_mut)
            .ok_or("VAL-CLIP-001 contracts are missing")?;
        if let Some(previous_contracts) =
            previous_artifact.get("contracts").and_then(Value::as_array)
        {
            for contract in previous_contracts {
                let task = contract.get("task_id").and_then(Value::as_str);
                if task != Some(CONTRACT_TASK) && task != Some(OWNER_RESULT_TASK) {
                    current_contracts.push(contract.clone());
                }
            }
        }
        let completed_tasks = artifact
            .get("task_results")
            .and_then(Value::as_object)
            .ok_or("VAL-CLIP-001 task results are missing")?
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        artifact
            .get_mut("remaining_tasks")
            .and_then(Value::as_array_mut)
            .ok_or("VAL-CLIP-001 remaining tasks are missing")?
            .retain(|task| {
                task.as_str()
                    .is_none_or(|task| !completed_tasks.contains(task))
            });
        let passed = artifact
            .get("task_results")
            .and_then(Value::as_object)
            .ok_or("VAL-CLIP-001 task results are missing")?
            .values()
            .try_fold(0_u64, |total, result| {
                total
                    .checked_add(
                        result
                            .get("passed")
                            .and_then(Value::as_u64)
                            .ok_or("VAL-CLIP-001 task result has no passed count")?,
                    )
                    .ok_or("VAL-CLIP-001 passed count overflowed")
            })?;
        artifact["summary"] = json!({"passed": passed, "failed": 0, "skipped": 0});
    }
    fs::create_dir_all(&artifact_directory)?;
    fs::write(&artifact_path, serde_json::to_vec_pretty(&artifact)?)?;
    Ok(())
}
