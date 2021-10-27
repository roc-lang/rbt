interface Rbt
  exposes [ Rbt, init ]
  imports [ Job.{ Job } ]

Rbt : { default : Job }

init : { default : Job } -> Rbt
init = \rbt -> rbt
