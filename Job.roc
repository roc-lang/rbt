interface Job
  exposes [ Job, job ]
  imports []

Job : [ @Job { command : Str, inputs : List Job, inputFiles : List Str, outputs : List Str } ]

job : { command : Str, inputs ? List Job, inputFiles ? List Str, outputs : List Str } -> Job
job = \{ command, outputs, inputs ? [], inputFiles ? [] } ->
   @Job { command, inputs, inputFiles, outputs }
