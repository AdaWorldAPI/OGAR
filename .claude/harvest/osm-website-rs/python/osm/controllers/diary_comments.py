"""@generated DO arm — faithful mirror of the `diary_comments` controller.
`osm.controllers.diary_comments.<is_a>(inp)` free functions (re-exported as
`osm.diary_comments`); standalone, not methods on the model. Call:
`osm.controllers.diary_comments.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def comment_params(inp: Input) -> Output:
    """`diary_comments:comment_params` — DO arm. Source: DiaryCommentsController#comment_params."""
    raise NotImplementedError("port DiaryCommentsController#comment_params")

def create(inp: Input) -> Output:
    """`diary_comments:create` — DO arm. Source: DiaryCommentsController#create."""
    raise NotImplementedError("port DiaryCommentsController#create")

def hide(inp: Input) -> Output:
    """`diary_comments:hide` — DO arm. Source: DiaryCommentsController#hide."""
    raise NotImplementedError("port DiaryCommentsController#hide")

def list(inp: Input) -> Output:
    """`diary_comments:list` — DO arm. Source: Users::DiaryCommentsController#index."""
    raise NotImplementedError("port Users::DiaryCommentsController#index")

def unhide(inp: Input) -> Output:
    """`diary_comments:unhide` — DO arm. Source: DiaryCommentsController#unhide."""
    raise NotImplementedError("port DiaryCommentsController#unhide")

