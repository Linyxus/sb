@main def app(): Unit =
  val s: String = null  // error: null cannot be assigned to String
  val n: Int = s.length
  println(n)
