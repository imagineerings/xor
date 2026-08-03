{ lib, stdenvNoCC }:

stdenvNoCC.mkDerivation {
  pname = "sim-comfy-backend-rocm-metadata";
  version = "6.1.0-abi1";

  dontUnpack = true;

  installPhase = ''
    runHook preInstall
    destination="$out/lib/sim/comfy-backends/rocm"
    mkdir -p "$destination/abi"
    cp ${../../../crates/comfy_backend_rocm/abi/symbols-v1.json} "$destination/abi/"
    cp ${../../../crates/comfy_backend_rocm/LICENSES} "$destination/"
    cp ${./package-policy.json} "$destination/"
    runHook postInstall
  '';

  passthru = {
    redistributesAmdRuntime = false;
    requiresPlatformPackageSignature = true;
  };

  meta = {
    description = "Reviewed Sim ROCm ABI metadata without AMD runtime redistribution";
    license = lib.licenses.gpl3Plus;
    platforms = [ "x86_64-linux" ];
  };
}
