// Lambdas, closures, eta-expansion
object Lambdas:
  val add: (Int, Int) => Int = _ + _
  val inc: Int => Int = _ + 1
  val isEven: Int => Boolean = _ % 2 == 0

  def applyTwice[A](f: A => A, x: A): A = f(f(x))

  def compose[A, B, C](f: B => C, g: A => B): A => C = a => f(g(a))

  val transform: List[Int] => List[String] =
    _.filter(_ > 0).map(_.toString)

  // Capturing outer variables
  def counter(start: Int): () => Int =
    var n = start
    () => { n += 1; n }
