{ lib, stdenvNoCC }:

stdenvNoCC.mkDerivation {
  pname = "zed-comfy-backend-mlu-metadata";
  version = "neuware1.20-abi1";

  dontUnpack = true;

  installPhase = ''
    runHook preInstall
    destination="$out/lib/zed/comfy-backends/mlu"
    mkdir -p "$destination/abi"
    cp ${../../../crates/comfy_backend_mlu/abi/symbols-v1.json} "$destination/abi/"
    cp ${../../../crates/comfy_backend_mlu/LICENSES} "$destination/"
    cp ${./package-policy.json} "$destination/"
    runHook postInstall
  '';

  passthru = {
    redistributesVendorRuntime = false;
    invokesVendorCompiler = false;
    releasePackager = "script/package-comfy-backend-mlu";
    cryptographicVerificationOwner = "comfy_runtime::NativeFfiRegistry";
    supportedTargets = [ "x86_64-linux" "aarch64-linux" ];
  };

  meta = {
    description = "Reviewed Zed Cambricon MLU ABI metadata without vendor runtime redistribution";
    license = lib.licenses.gpl3Plus;
    platforms = [ "x86_64-linux" "aarch64-linux" ];
  };
}
