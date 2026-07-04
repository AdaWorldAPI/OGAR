"""@generated DO arm — faithful mirror of the `profile_sections` controller.
`osm.controllers.profile_sections.<is_a>(inp)` free functions (re-exported as
`osm.profile_sections`); standalone, not methods on the model. Call:
`osm.controllers.profile_sections.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`profile_sections:show` — DO arm. Source: Profiles::ProfileSectionsController#show."""
    raise NotImplementedError("port Profiles::ProfileSectionsController#show")

def update(inp: Input) -> Output:
    """`profile_sections:update` — DO arm. Source: Profiles::ProfileSectionsController#update."""
    raise NotImplementedError("port Profiles::ProfileSectionsController#update")

