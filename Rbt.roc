interface Rbt
  exposes [ Rbt, init ]
  imports []

Rbt : { default : Job }

init : { default : Job } -> Rbt
init = \rbt -> rbt
