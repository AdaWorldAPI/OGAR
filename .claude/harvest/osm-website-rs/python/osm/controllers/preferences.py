"""@generated DO arm — faithful mirror of the `preferences` controller.
`osm.controllers.preferences.<is_a>(inp)` free functions (re-exported as
`osm.preferences`); standalone, not methods on the model. Call:
`osm.controllers.preferences.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`preferences:show` — DO arm. Source: Preferences::PreferencesController#show."""
    raise NotImplementedError("port Preferences::PreferencesController#show")

def update(inp: Input) -> Output:
    """`preferences:update` — DO arm. Source: Preferences::PreferencesController#update."""
    raise NotImplementedError("port Preferences::PreferencesController#update")

