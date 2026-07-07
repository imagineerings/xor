{ inputs, ... }:
{
  perSystem =
    {
      pkgs,
      lib,
      system,
      ...
    }:
    let
      mkSim = import ../toolchain.nix { inherit inputs; };
      sim-editor = mkSim pkgs;
    in
    {
      packages = {
        default = sim-editor;
        debug = sim-editor.override { profile = "dev"; };
      };
    }
    // lib.optionalAttrs (lib.hasSuffix "linux" system) {
      checks.a11y-test = import ../tests/a11y.nix {
        inherit pkgs inputs;
      };
    };
}
