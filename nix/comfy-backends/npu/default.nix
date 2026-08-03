{ lib, stdenvNoCC }:

stdenvNoCC.mkDerivation {
  pname = "sim-comfy-backend-npu-metadata";
  version = "cann-8.0-rc3-abi1";

  dontUnpack = true;

  installPhase = ''
    runHook preInstall
    destination="$out/lib/sim/comfy-backends/npu"
    mkdir -p "$destination/abi"
    cp ${../../../crates/comfy_backend_npu/abi/symbols-v1.json} "$destination/abi/"
    cp ${../../../crates/comfy_backend_npu/LICENSES} "$destination/"
    cp ${./package-policy.json} "$destination/"
    runHook postInstall
  '';

  passthru = {
    redistributesCann = false;
    requiredExternalLibraries = [ "libascendcl.so" "libruntime.so" ];
    discoveryOrder = [ "COMFY_ASCEND_ROOT" "ASCEND_HOME_PATH" "signed_package_roots" ];
    releasePackager = "script/package-comfy-backend-npu";
    cryptographicVerificationOwner = "comfy_runtime::NativeFfiRegistry plus signer-bound NPU package key";
  };

  meta = {
    description = "Reviewed Sim Huawei Ascend NPU ABI metadata without CANN redistribution";
    license = lib.licenses.gpl3Plus;
    platforms = [ "aarch64-linux" "x86_64-linux" ];
  };
}
