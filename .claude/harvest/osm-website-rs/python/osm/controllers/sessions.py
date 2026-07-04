"""@generated DO arm — faithful mirror of the `sessions` controller.
`osm.controllers.sessions.<is_a>(inp)` free functions (re-exported as
`osm.sessions`); standalone, not methods on the model. Call:
`osm.controllers.sessions.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`sessions:create` — DO arm. Source: SessionsController#create."""
    raise NotImplementedError("port SessionsController#create")

def delete(inp: Input) -> Output:
    """`sessions:delete` — DO arm. Source: SessionsController#destroy."""
    raise NotImplementedError("port SessionsController#destroy")

def new_form(inp: Input) -> Output:
    """`sessions:new_form` — DO arm. Source: SessionsController#new."""
    raise NotImplementedError("port SessionsController#new")

def password_authentication(inp: Input) -> Output:
    """`sessions:password_authentication` — DO arm. Source: SessionsController#password_authentication."""
    raise NotImplementedError("port SessionsController#password_authentication")

