"""@generated DO arm — faithful mirror of the `changeset_comments` controller.
`osm.controllers.changeset_comments.<is_a>(inp)` free functions (re-exported as
`osm.changeset_comments`); standalone, not methods on the model. Call:
`osm.controllers.changeset_comments.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`changeset_comments:create` — DO arm. Source: Api::ChangesetCommentsController#create."""
    raise NotImplementedError("port Api::ChangesetCommentsController#create")

def list(inp: Input) -> Output:
    """`changeset_comments:list` — DO arm. Sources (canonical tile): Api::ChangesetCommentsController#index, Users::ChangesetCommentsController#index."""
    raise NotImplementedError("port Api::ChangesetCommentsController#index")

def rate_limit_exceeded(inp: Input) -> Output:
    """`changeset_comments:rate_limit_exceeded?` — DO arm. Source: Api::ChangesetCommentsController#rate_limit_exceeded?."""
    raise NotImplementedError("port Api::ChangesetCommentsController#rate_limit_exceeded?")

