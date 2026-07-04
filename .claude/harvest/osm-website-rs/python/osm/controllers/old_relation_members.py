"""@generated DO arm — faithful mirror of the `old_relation_members` controller.
`osm.controllers.old_relation_members.<is_a>(inp)` free functions (re-exported as
`osm.old_relation_members`); standalone, not methods on the model. Call:
`osm.controllers.old_relation_members.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`old_relation_members:show` — DO arm. Source: OldRelationMembersController#show."""
    raise NotImplementedError("port OldRelationMembersController#show")

