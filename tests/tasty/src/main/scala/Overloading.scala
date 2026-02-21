// Method overloading
object Overloading:
  def process(x: Int): String = s"int: $x"
  def process(x: String): String = s"string: $x"
  def process(x: Double): String = s"double: $x"
  def process(x: Int, y: Int): String = s"pair: ($x, $y)"
  def process(xs: List[Int]): String = s"list: ${xs.mkString(", ")}"
