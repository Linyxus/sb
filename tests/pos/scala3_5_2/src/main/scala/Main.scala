import scala.util.boundary
import scala.util.boundary.break

def findFirst[A](xs: List[A])(pred: A => Boolean): Option[A] =
  boundary:
    for x <- xs do
      if pred(x) then break(Some(x))
    None

@main def app(): Unit =
  val nums = List(2, 4, 7, 10, 13, 16)
  val firstOdd = findFirst(nums)(_ % 2 != 0)
  println(s"First odd: $firstOdd")
  val over20 = findFirst(nums)(_ > 20)
  println(s"Over 20: $over20")
