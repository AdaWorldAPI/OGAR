"""@generated DO arm — faithful mirror of the `issue_comments` controller.
`osm.controllers.issue_comments.<is_a>(inp)` free functions (re-exported as
`osm.issue_comments`); standalone, not methods on the model. Call:
`osm.controllers.issue_comments.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`issue_comments:create` — DO arm. Source: IssueCommentsController#create."""
    raise NotImplementedError("port IssueCommentsController#create")

def issue_comment_params(inp: Input) -> Output:
    """`issue_comments:issue_comment_params` — DO arm. Source: IssueCommentsController#issue_comment_params."""
    raise NotImplementedError("port IssueCommentsController#issue_comment_params")

def reassign_issue(inp: Input) -> Output:
    """`issue_comments:reassign_issue` — DO arm. Source: IssueCommentsController#reassign_issue."""
    raise NotImplementedError("port IssueCommentsController#reassign_issue")

