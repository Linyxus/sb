class Stack[+A](private val elements: List[A]):
  def push[B >: A](elem: B): Stack[B] = Stack(elem :: elements)
  def pop: (A, Stack[A]) = (elements.head, Stack(elements.tail))
  def isEmpty: Boolean = elements.isEmpty
  def toList: List[A] = elements
  override def toString: String = s"Stack(${elements.mkString(", ")})"

object Stack:
  def empty[A]: Stack[A] = Stack(Nil)

trait Transformer[-A, +B]:
  def transform(a: A): B

def findMin[A: Ordering](items: List[A]): A =
  items.min

def pairUp[A, B](as: List[A], bs: List[B]): List[(A, B)] =
  as.zip(bs)

@main def typeParamsMain(): Unit =
  val s1 = Stack.empty[Int].push(1).push(2).push(3)
  println(s"Stack: $s1")
  val (top, s2) = s1.pop
  println(s"Popped $top, remaining: $s2")

  // Covariance: Stack[Int] usable as Stack[Any]
  val anyStack: Stack[Any] = s2.push("hello")
  println(s"Mixed stack: $anyStack")

  // Contravariance in Transformer
  val stringify: Transformer[Any, String] = new Transformer[Any, String]:
    def transform(a: Any): String = a.toString.toUpperCase
  val intTransformer: Transformer[Int, String] = stringify
  println(intTransformer.transform(42))

  println(s"Min of [3,1,4,1,5]: ${findMin(List(3, 1, 4, 1, 5))}")
  println(s"Pairs: ${pairUp(List("a", "b", "c"), List(1, 2, 3))}")
