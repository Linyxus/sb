// Case classes and copy
case class Point(x: Double, y: Double):
  def distanceTo(other: Point): Double =
    math.sqrt(math.pow(x - other.x, 2) + math.pow(y - other.y, 2))

case class Color(r: Int, g: Int, b: Int)

case class Pair[A, B](first: A, second: B)
