enum Color(val hex: String):
  case Red extends Color("#FF0000")
  case Green extends Color("#00FF00")
  case Blue extends Color("#0000FF")

enum Shape:
  case Circle(radius: Double)
  case Rectangle(width: Double, height: Double)

def area(s: Shape): Double = s match
  case Shape.Circle(r) => math.Pi * r * r
  case Shape.Rectangle(w, h) => w * h

def describe(c: Color): String = c match
  case Color.Red => s"Red (${c.hex})"
  case Color.Green => s"Green (${c.hex})"
  case Color.Blue => s"Blue (${c.hex})"

@main def enumsMain(): Unit =
  Color.values.foreach(c => println(describe(c)))
  val shapes = List(Shape.Circle(5.0), Shape.Rectangle(3.0, 4.0))
  shapes.foreach(s => println(s"Area of $s = ${area(s)}"))
