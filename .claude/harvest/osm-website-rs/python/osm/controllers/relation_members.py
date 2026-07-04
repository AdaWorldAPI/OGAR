"""@generated DO arm — faithful mirror of the `relation_members` controller.
`osm.controllers.relation_members.<is_a>(inp)` free functions (re-exported as
`osm.relation_members`); standalone, not methods on the model. Call:
`osm.controllers.relation_members.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`relation_members:show` — DO arm. Source: RelationMembersController#show."""
    raise NotImplementedError("port RelationMembersController#show")

