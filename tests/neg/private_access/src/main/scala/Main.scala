class Secret:
  private val code = 42

@main def app(): Unit =
  val s = Secret()
  println(s.code)  // private access
