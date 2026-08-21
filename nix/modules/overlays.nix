{ inputs, ... }:
{
  flake.overlays.default =
    final: _:
    let
      mkSim = import ../toolchain.nix { inherit inputs; };
    in
    {
      zed-editor = mkSim final;
    };
}
