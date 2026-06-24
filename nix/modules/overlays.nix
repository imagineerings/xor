{ inputs, ... }:
{
  flake.overlays.default =
    final: _:
    let
      mkBaymax = import ../toolchain.nix { inherit inputs; };
    in
    {
      baymax-editor = mkBaymax final;
    };
}
