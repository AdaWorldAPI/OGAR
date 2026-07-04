"""@generated DO arm — faithful mirror of the `dashboards` controller.
`osm.controllers.dashboards.<is_a>(inp)` free functions (re-exported as
`osm.dashboards`); standalone, not methods on the model. Call:
`osm.controllers.dashboards.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`dashboards:show` — DO arm. Source: DashboardsController#show."""
    raise NotImplementedError("port DashboardsController#show")

