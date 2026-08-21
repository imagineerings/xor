{ lib, stdenvNoCC }:

stdenvNoCC.mkDerivation {
  pname = "zed-comfy-backend-directml-metadata";
  version = "1.13.1-abi1";

  dontUnpack = true;

  installPhase = ''
    runHook preInstall
    destination="$out/lib/zed/comfy-backends/directml"
    mkdir -p "$destination/abi"
    cp ${../../../crates/comfy_backend_directml/abi/symbols-v1.json} "$destination/abi/"
    cp ${../../../crates/comfy_backend_directml/LICENSES} "$destination/"
    cp ${./package-policy.json} "$destination/"
    runHook postInstall
  '';

  passthru = {
    redistributesMicrosoftDirectML = false;
    releasePackager = "script/package-comfy-backend-directml";
    approvedRedistributableVersion = "1.13.1";
    cryptographicVerificationOwner = "comfy_runtime::NativeFfiRegistry";
  };

  meta = {
    description = "Reviewed Zed DirectML 1.13 ABI metadata without binary redistribution";
    license = lib.licenses.gpl3Plus;
    platforms = [ "aarch64-windows" "x86_64-windows" ];
  };
}
