interface Command
  exposes [ Command, exec ]
  imports [ Tool.{ Tool } ]

Command : { tool : Tool, args : List Str }

exec : Tool, List Str -> Command
exec = \tool, args -> { tool, args }
