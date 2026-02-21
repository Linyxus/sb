// Enums: simple and parameterized
enum Direction:
  case North, South, East, West

enum Planet(val mass: Double, val radius: Double):
  case Mercury extends Planet(3.303e+23, 2.4397e6)
  case Venus extends Planet(4.869e+24, 6.0518e6)
  case Earth extends Planet(5.976e+24, 6.37814e6)

  def surfaceGravity: Double = 6.67300e-11 * mass / (radius * radius)

enum Expr:
  case Lit(value: Int)
  case Add(left: Expr, right: Expr)
  case Mul(left: Expr, right: Expr)
