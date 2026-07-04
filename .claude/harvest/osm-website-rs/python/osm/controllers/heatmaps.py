"""@generated DO arm — faithful mirror of the `heatmaps` controller.
`osm.controllers.heatmaps.<is_a>(inp)` free functions (re-exported as
`osm.heatmaps`); standalone, not methods on the model. Call:
`osm.controllers.heatmaps.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`heatmaps:show` — DO arm. Source: Users::HeatmapsController#show."""
    raise NotImplementedError("port Users::HeatmapsController#show")

def update_profile(inp: Input) -> Output:
    """`heatmaps:update_profile` — DO arm. Source: Profiles::HeatmapsController#update_profile."""
    raise NotImplementedError("port Profiles::HeatmapsController#update_profile")

