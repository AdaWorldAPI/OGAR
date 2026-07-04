"""@generated DO arm — faithful mirror of the `companies` controller.
`osm.controllers.companies.<is_a>(inp)` free functions (re-exported as
`osm.companies`); standalone, not methods on the model. Call:
`osm.controllers.companies.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def update_profile(inp: Input) -> Output:
    """`companies:update_profile` — DO arm. Source: Profiles::CompaniesController#update_profile."""
    raise NotImplementedError("port Profiles::CompaniesController#update_profile")

