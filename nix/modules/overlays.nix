{ inputs, ... }:
{
  flake.overlays.default =
    final: _:
    let
      mkSim = import ../toolchain.nix { inherit inputs; };
    in
    {
      sim-editor = mkSim final;
    };
}
