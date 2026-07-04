"""@generated DO arm — faithful mirror of the `locations` controller.
`osm.controllers.locations.<is_a>(inp)` free functions (re-exported as
`osm.locations`); standalone, not methods on the model. Call:
`osm.controllers.locations.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`locations:show` — DO arm. Source: Profiles::LocationsController#show."""
    raise NotImplementedError("port Profiles::LocationsController#show")

def update_profile(inp: Input) -> Output:
    """`locations:update_profile` — DO arm. Source: Profiles::LocationsController#update_profile."""
    raise NotImplementedError("port Profiles::LocationsController#update_profile")

