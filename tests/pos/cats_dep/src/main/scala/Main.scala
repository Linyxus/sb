import cats.*
import cats.syntax.all.*

case class Box[A](value: A)

given Functor[Box] with
  def map[A, B](fa: Box[A])(f: A => B): Box[B] = Box(f(fa.value))

@main def catsMain(): Unit =
  // Functor
  val box = Box(42)
  val mapped = Functor[Box].map(box)(_ + 1)
  println(s"Functor map: $mapped")

  // Semigroup combine
  val combined = "Hello" |+| " " |+| "World"
  println(s"Semigroup: $combined")

  // Option as Monad
  val result = for
    a <- Option(10)
    b <- Option(20)
    c <- Option(a + b)
  yield c * 2
  println(s"Monad result: $result")

  // Traverse
  val opts: List[Option[Int]] = List(Some(1), Some(2), Some(3))
  val sequenced: Option[List[Int]] = opts.sequence
  println(s"Sequence: $sequenced")

  // Show
  import cats.Show
  given Show[Box[Int]] with
    def show(b: Box[Int]): String = s"[Box: ${b.value}]"
  println(s"Show: ${box.show}")
