trait Printable[A]:
  extension (a: A) def prettyPrint: String

trait Combinable[A]:
  def combine(x: A, y: A): A
  def empty: A

given Printable[Int] with
  extension (a: Int) def prettyPrint: String = s"Int($a)"

given Printable[String] with
  extension (a: String) def prettyPrint: String = s"Str(\"$a\")"

given Combinable[Int] with
  def combine(x: Int, y: Int): Int = x + y
  def empty: Int = 0

given Combinable[String] with
  def combine(x: String, y: String): String = x + y
  def empty: String = ""

def printAll[A: Printable](items: List[A]): Unit =
  items.foreach(a => println(a.prettyPrint))

def combineAll[A](items: List[A])(using c: Combinable[A]): A =
  items.foldLeft(c.empty)(c.combine)

@main def givensMain(): Unit =
  printAll(List(1, 2, 3))
  printAll(List("hello", "world"))
  println(s"Sum: ${combineAll(List(1, 2, 3, 4))}")
  println(s"Concat: ${combineAll(List("a", "b", "c"))}")
