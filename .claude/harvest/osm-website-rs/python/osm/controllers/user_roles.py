"""@generated DO arm — faithful mirror of the `user_roles` controller.
`osm.controllers.user_roles.<is_a>(inp)` free functions (re-exported as
`osm.user_roles`); standalone, not methods on the model. Call:
`osm.controllers.user_roles.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`user_roles:create` — DO arm. Source: UserRolesController#create."""
    raise NotImplementedError("port UserRolesController#create")

def delete(inp: Input) -> Output:
    """`user_roles:delete` — DO arm. Source: UserRolesController#destroy."""
    raise NotImplementedError("port UserRolesController#destroy")

def in_role(inp: Input) -> Output:
    """`user_roles:in_role` — DO arm. Source: UserRolesController#in_role."""
    raise NotImplementedError("port UserRolesController#in_role")

def not_in_role(inp: Input) -> Output:
    """`user_roles:not_in_role` — DO arm. Source: UserRolesController#not_in_role."""
    raise NotImplementedError("port UserRolesController#not_in_role")

def require_valid_role(inp: Input) -> Output:
    """`user_roles:require_valid_role` — DO arm. Source: UserRolesController#require_valid_role."""
    raise NotImplementedError("port UserRolesController#require_valid_role")

