{ lib, stdenvNoCC }:

stdenvNoCC.mkDerivation {
  pname = "zed-comfy-backend-corex-metadata";
  version = "ixrt-0.8-abi1-provenance-blocked";

  dontUnpack = true;

  installPhase = ''
    runHook preInstall
    destination="$out/lib/zed/comfy-backends/corex"
    mkdir -p "$destination/abi"
    cp ${../../../crates/comfy_backend_corex/abi/symbols-v1.json} "$destination/abi/"
    cp ${../../../crates/comfy_backend_corex/LICENSES} "$destination/"
    cp ${./package-policy.json} "$destination/"
    runHook postInstall
  '';

  passthru = {
    redistributesVendorRuntime = false;
    requiredExternalLibraries = [ "libixblas.so" "libixrt.so" ];
    discoveryOrder = [ "COMFY_COREX_ROOT" "IXRT_HOME" "signed_package_roots" ];
    releasePackager = "script/package-comfy-backend-corex";
    runtimeLoadingEnabled = false;
    cryptographicVerificationOwner = "comfy_runtime::NativeFfiRegistry plus signer-bound CoreX package key";
  };

  meta = {
    description = "Fail-closed Zed CoreX IXRT 0.8 ABI metadata without vendor redistribution";
    license = lib.licenses.gpl3Plus;
    platforms = [ "x86_64-linux" ];
  };
}
