"""@generated DO arm — faithful mirror of the `messages` controller.
`osm.controllers.messages.<is_a>(inp)` free functions (re-exported as
`osm.messages`); standalone, not methods on the model. Call:
`osm.controllers.messages.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`messages:create` — DO arm. Sources (canonical tile): Api::MessagesController#create, MessagesController#create."""
    raise NotImplementedError("port Api::MessagesController#create")

def delete(inp: Input) -> Output:
    """`messages:delete` — DO arm. Sources (canonical tile): Api::MessagesController#destroy, MessagesController#destroy."""
    raise NotImplementedError("port Api::MessagesController#destroy")

def message_params(inp: Input) -> Output:
    """`messages:message_params` — DO arm. Source: MessagesController#message_params."""
    raise NotImplementedError("port MessagesController#message_params")

def new_form(inp: Input) -> Output:
    """`messages:new_form` — DO arm. Source: MessagesController#new."""
    raise NotImplementedError("port MessagesController#new")

def show(inp: Input) -> Output:
    """`messages:show` — DO arm. Sources (canonical tile): Api::MessagesController#show, MessagesController#show."""
    raise NotImplementedError("port Api::MessagesController#show")

def update(inp: Input) -> Output:
    """`messages:update` — DO arm. Source: Api::MessagesController#update."""
    raise NotImplementedError("port Api::MessagesController#update")

