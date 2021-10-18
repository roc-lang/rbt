(module
  (func (export "add") (param $n1 i32) (param $n2 i32) (result i32)
    get_local $n1
    get_local $n2
    i32.add
  )
)
