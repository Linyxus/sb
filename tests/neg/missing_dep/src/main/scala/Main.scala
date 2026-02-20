import cats.syntax.all.*

@main def app(): Unit =
  val x = List(1, 2, 3).map(_ + 1)
  println(x.show)
