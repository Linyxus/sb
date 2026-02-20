sealed trait Expr[A]:
  def eval: A = this match
    case Lit(n)      => n
    case Add(l, r)   => l.eval + r.eval
    case Mul(l, r)   => l.eval * r.eval
    case Neg(e)      => -e.eval

case class Lit(n: Int) extends Expr[Int]
case class Add(l: Expr[Int], r: Expr[Int]) extends Expr[Int]
case class Mul(l: Expr[Int], r: Expr[Int]) extends Expr[Int]
case class Neg(e: Expr[Int]) extends Expr[Int]

@main def app(): Unit =
  // (3 + 4) * -(2 + 1)
  val expr = Mul(Add(Lit(3), Lit(4)), Neg(Add(Lit(2), Lit(1))))
  println(s"Result: ${expr.eval}")
