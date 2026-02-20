@main def app(): Unit =
  val buf: ArrayBuffer[Int] = ArrayBuffer(1, 2, 3) // missing import
  println(buf)
