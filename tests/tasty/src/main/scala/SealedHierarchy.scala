// Sealed trait hierarchies
sealed trait Shape:
  def area: Double
  def perimeter: Double

final case class Circle(radius: Double) extends Shape:
  def area: Double = math.Pi * radius * radius
  def perimeter: Double = 2 * math.Pi * radius

final case class Rectangle(width: Double, height: Double) extends Shape:
  def area: Double = width * height
  def perimeter: Double = 2 * (width + height)

final case class Triangle(a: Double, b: Double, c: Double) extends Shape:
  def area: Double =
    val s = (a + b + c) / 2
    math.sqrt(s * (s - a) * (s - b) * (s - c))
  def perimeter: Double = a + b + c

object ShapeOps:
  def describe(s: Shape): String = s match
    case Circle(r) => s"circle r=$r"
    case Rectangle(w, h) => s"rect ${w}x$h"
    case Triangle(a, b, c) => s"tri $a,$b,$c"
