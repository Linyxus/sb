sealed trait Shape
case class Circle(r: Double) extends Shape
case class Square(s: Double) extends Shape
case class Triangle(a: Double, b: Double, c: Double) extends Shape

@main def app(): Unit =
  val s: Shape = Circle(1.0)
  val desc = s match  // non-exhaustive: missing Triangle
    case Circle(r) => s"circle $r"
    case Square(s) => s"square $s"
  println(desc)
