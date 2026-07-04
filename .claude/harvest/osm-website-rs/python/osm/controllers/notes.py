"""@generated DO arm — faithful mirror of the `notes` controller.
`osm.controllers.notes.<is_a>(inp)` free functions (re-exported as
`osm.notes`); standalone, not methods on the model. Call:
`osm.controllers.notes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def add_comment(inp: Input) -> Output:
    """`notes:add_comment` — DO arm. Source: Api::NotesController#add_comment."""
    raise NotImplementedError("port Api::NotesController#add_comment")

def author_info(inp: Input) -> Output:
    """`notes:author_info` — DO arm. Source: Api::NotesController#author_info."""
    raise NotImplementedError("port Api::NotesController#author_info")

def bbox_condition(inp: Input) -> Output:
    """`notes:bbox_condition` — DO arm. Source: Api::NotesController#bbox_condition."""
    raise NotImplementedError("port Api::NotesController#bbox_condition")

def close(inp: Input) -> Output:
    """`notes:close` — DO arm. Source: Api::NotesController#close."""
    raise NotImplementedError("port Api::NotesController#close")

def closed_condition(inp: Input) -> Output:
    """`notes:closed_condition` — DO arm. Source: Api::NotesController#closed_condition."""
    raise NotImplementedError("port Api::NotesController#closed_condition")

def comment(inp: Input) -> Output:
    """`notes:comment` — DO arm. Source: Api::NotesController#comment."""
    raise NotImplementedError("port Api::NotesController#comment")

def create(inp: Input) -> Output:
    """`notes:create` — DO arm. Source: Api::NotesController#create."""
    raise NotImplementedError("port Api::NotesController#create")

def delete(inp: Input) -> Output:
    """`notes:delete` — DO arm. Source: Api::NotesController#destroy."""
    raise NotImplementedError("port Api::NotesController#destroy")

def feed(inp: Input) -> Output:
    """`notes:feed` — DO arm. Source: Api::NotesController#feed."""
    raise NotImplementedError("port Api::NotesController#feed")

def list(inp: Input) -> Output:
    """`notes:list` — DO arm. Sources (canonical tile): Api::NotesController#index, NotesController#index."""
    raise NotImplementedError("port Api::NotesController#index")

def new_form(inp: Input) -> Output:
    """`notes:new_form` — DO arm. Source: NotesController#new."""
    raise NotImplementedError("port NotesController#new")

def reopen(inp: Input) -> Output:
    """`notes:reopen` — DO arm. Source: Api::NotesController#reopen."""
    raise NotImplementedError("port Api::NotesController#reopen")

def search(inp: Input) -> Output:
    """`notes:search` — DO arm. Source: Api::NotesController#search."""
    raise NotImplementedError("port Api::NotesController#search")

def show(inp: Input) -> Output:
    """`notes:show` — DO arm. Sources (canonical tile): Api::NotesController#show, NotesController#show."""
    raise NotImplementedError("port Api::NotesController#show")

