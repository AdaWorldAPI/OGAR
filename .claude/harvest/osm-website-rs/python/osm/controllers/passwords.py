"""@generated DO arm — faithful mirror of the `passwords` controller.
`osm.controllers.passwords.<is_a>(inp)` free functions (re-exported as
`osm.passwords`); standalone, not methods on the model. Call:
`osm.controllers.passwords.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`passwords:create` — DO arm. Source: PasswordsController#create."""
    raise NotImplementedError("port PasswordsController#create")

def edit(inp: Input) -> Output:
    """`passwords:edit` — DO arm. Source: PasswordsController#edit."""
    raise NotImplementedError("port PasswordsController#edit")

def new_form(inp: Input) -> Output:
    """`passwords:new_form` — DO arm. Source: PasswordsController#new."""
    raise NotImplementedError("port PasswordsController#new")

def update(inp: Input) -> Output:
    """`passwords:update` — DO arm. Source: PasswordsController#update."""
    raise NotImplementedError("port PasswordsController#update")

