// Pattern matching: guards, nested, typed, alternatives
object PatternMatch:
  def describe(x: Any): String = x match
    case i: Int if i > 0 => s"positive int: $i"
    case i: Int => s"non-positive int: $i"
    case s: String => s"string of length ${s.length}"
    case (a, b) => s"pair: ($a, $b)"
    case list: List[?] => s"list of ${list.length}"
    case null => "null"
    case _ => "something else"

  def eval(e: Expr): Int = e match
    case Expr.Lit(v) => v
    case Expr.Add(l, r) => eval(l) + eval(r)
    case Expr.Mul(l, r) => eval(l) * eval(r)
