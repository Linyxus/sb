@main def app(): Unit =
  val x = NonExistentClass.doSomething()
  println(x)
