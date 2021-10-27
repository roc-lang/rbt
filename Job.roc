interface Job
  exposes [ Job, job ]
  imports [ Command.{ Command } ]

Job : [ @Job { command : Command, inputs : List Job, inputFiles : List Str, outputs : List Str } ]

job : { command : Command, inputs ? List Job, inputFiles ? List Str, outputs : List Str } -> Job
job = \{ command, outputs, inputs ? [], inputFiles ? [] } ->
   @Job { command, inputs, inputFiles, outputs }
