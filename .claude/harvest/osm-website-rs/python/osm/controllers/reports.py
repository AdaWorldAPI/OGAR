"""@generated DO arm — faithful mirror of the `reports` controller.
`osm.controllers.reports.<is_a>(inp)` free functions (re-exported as
`osm.reports`); standalone, not methods on the model. Call:
`osm.controllers.reports.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`reports:create` — DO arm. Source: ReportsController#create."""
    raise NotImplementedError("port ReportsController#create")

def create_new_report_params(inp: Input) -> Output:
    """`reports:create_new_report_params` — DO arm. Source: ReportsController#create_new_report_params."""
    raise NotImplementedError("port ReportsController#create_new_report_params")

def default_assigned_role(inp: Input) -> Output:
    """`reports:default_assigned_role` — DO arm. Source: ReportsController#default_assigned_role."""
    raise NotImplementedError("port ReportsController#default_assigned_role")

def issue_params(inp: Input) -> Output:
    """`reports:issue_params` — DO arm. Source: ReportsController#issue_params."""
    raise NotImplementedError("port ReportsController#issue_params")

def new_form(inp: Input) -> Output:
    """`reports:new_form` — DO arm. Source: ReportsController#new."""
    raise NotImplementedError("port ReportsController#new")

def report_params(inp: Input) -> Output:
    """`reports:report_params` — DO arm. Source: ReportsController#report_params."""
    raise NotImplementedError("port ReportsController#report_params")

def required_new_report_params_present(inp: Input) -> Output:
    """`reports:required_new_report_params_present?` — DO arm. Source: ReportsController#required_new_report_params_present?."""
    raise NotImplementedError("port ReportsController#required_new_report_params_present?")

