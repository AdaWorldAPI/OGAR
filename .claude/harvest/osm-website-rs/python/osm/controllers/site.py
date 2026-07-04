"""@generated DO arm — faithful mirror of the `site` controller.
`osm.controllers.site.<is_a>(inp)` free functions (re-exported as
`osm.site`); standalone, not methods on the model. Call:
`osm.controllers.site.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def about(inp: Input) -> Output:
    """`site:about` — DO arm. Source: SiteController#about."""
    raise NotImplementedError("port SiteController#about")

def communities(inp: Input) -> Output:
    """`site:communities` — DO arm. Source: SiteController#communities."""
    raise NotImplementedError("port SiteController#communities")

def copyright(inp: Input) -> Output:
    """`site:copyright` — DO arm. Source: SiteController#copyright."""
    raise NotImplementedError("port SiteController#copyright")

def edit(inp: Input) -> Output:
    """`site:edit` — DO arm. Source: SiteController#edit."""
    raise NotImplementedError("port SiteController#edit")

def export(inp: Input) -> Output:
    """`site:export` — DO arm. Source: SiteController#export."""
    raise NotImplementedError("port SiteController#export")

def help(inp: Input) -> Output:
    """`site:help` — DO arm. Source: SiteController#help."""
    raise NotImplementedError("port SiteController#help")

def id(inp: Input) -> Output:
    """`site:id` — DO arm. Source: SiteController#id."""
    raise NotImplementedError("port SiteController#id")

def list(inp: Input) -> Output:
    """`site:list` — DO arm. Source: SiteController#index."""
    raise NotImplementedError("port SiteController#index")

def offline(inp: Input) -> Output:
    """`site:offline` — DO arm. Source: SiteController#offline."""
    raise NotImplementedError("port SiteController#offline")

def permalink(inp: Input) -> Output:
    """`site:permalink` — DO arm. Source: SiteController#permalink."""
    raise NotImplementedError("port SiteController#permalink")

def preview(inp: Input) -> Output:
    """`site:preview` — DO arm. Source: SiteController#preview."""
    raise NotImplementedError("port SiteController#preview")

def redirect_browse_params(inp: Input) -> Output:
    """`site:redirect_browse_params` — DO arm. Source: SiteController#redirect_browse_params."""
    raise NotImplementedError("port SiteController#redirect_browse_params")

def redirect_map_params(inp: Input) -> Output:
    """`site:redirect_map_params` — DO arm. Source: SiteController#redirect_map_params."""
    raise NotImplementedError("port SiteController#redirect_map_params")

def welcome(inp: Input) -> Output:
    """`site:welcome` — DO arm. Source: SiteController#welcome."""
    raise NotImplementedError("port SiteController#welcome")

