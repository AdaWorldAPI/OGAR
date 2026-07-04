"""@generated DO arm — faithful mirror of the `basic_preferences` controller.
`osm.controllers.basic_preferences.<is_a>(inp)` free functions (re-exported as
`osm.basic_preferences`); standalone, not methods on the model. Call:
`osm.controllers.basic_preferences.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def update_preferences(inp: Input) -> Output:
    """`basic_preferences:update_preferences` — DO arm. Source: Preferences::BasicPreferencesController#update_preferences."""
    raise NotImplementedError("port Preferences::BasicPreferencesController#update_preferences")

