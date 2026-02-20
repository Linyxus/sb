def add(a: Int, b: Int): Int = a + b

@main def app(): Unit =
  val result = add(1, 2, 3)  // too many arguments
  println(result)
