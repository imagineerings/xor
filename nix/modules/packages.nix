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
      mkBaymax = import ../toolchain.nix { inherit inputs; };
      baymax-editor = mkBaymax pkgs;
    in
    {
      packages = {
        default = baymax-editor;
        debug = baymax-editor.override { profile = "dev"; };
      };
    }
    // lib.optionalAttrs (lib.hasSuffix "linux" system) {
      checks.a11y-test = import ../tests/a11y.nix {
        inherit pkgs inputs;
      };
    };
}
