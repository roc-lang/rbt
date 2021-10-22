interface Tool
  exposes [ Tool, tool, systemTool ]
  imports []

Tool : Str

systemTool : Str -> Tool
systemTool = \name -> name

tool : Job, Str -> Tool
tool = \job, outputName ->
  "TODO"
