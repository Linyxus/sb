// Given instances and using clauses
trait Ordering[A]:
  def compare(a: A, b: A): Int

object Givens:
  given intOrdering: Ordering[Int] with
    def compare(a: Int, b: Int): Int = a - b

  given stringOrdering: Ordering[String] with
    def compare(a: String, b: String): Int = a.compareTo(b)

  given listOrdering[A](using ord: Ordering[A]): Ordering[List[A]] with
    def compare(a: List[A], b: List[A]): Int =
      (a, b) match
        case (Nil, Nil) => 0
        case (Nil, _) => -1
        case (_, Nil) => 1
        case (x :: xs, y :: ys) =>
          val c = ord.compare(x, y)
          if c != 0 then c else compare(xs, ys)

  def max[A](a: A, b: A)(using ord: Ordering[A]): A =
    if ord.compare(a, b) >= 0 then a else b
