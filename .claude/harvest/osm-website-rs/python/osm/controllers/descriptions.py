"""@generated DO arm — faithful mirror of the `descriptions` controller.
`osm.controllers.descriptions.<is_a>(inp)` free functions (re-exported as
`osm.descriptions`); standalone, not methods on the model. Call:
`osm.controllers.descriptions.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def update_profile(inp: Input) -> Output:
    """`descriptions:update_profile` — DO arm. Source: Profiles::DescriptionsController#update_profile."""
    raise NotImplementedError("port Profiles::DescriptionsController#update_profile")

