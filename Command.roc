interface Command
  exposes [ Command, exec ]
  imports []

Command : { tool : Tool, args : List Str }

exec : Tool, List Str -> Command
exec = \tool, args -> { tool, args }
