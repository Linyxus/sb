def compose[A, B, C](f: B => C, g: A => B): A => C =
  a => f(g(a))

def curry[A, B, C](f: (A, B) => C): A => B => C =
  a => b => f(a, b)

def twice[A](f: A => A): A => A =
  a => f(f(a))

def pipeline[A](value: A, transforms: List[A => A]): A =
  transforms.foldLeft(value)((acc, f) => f(acc))

@main def higherOrderMain(): Unit =
  val inc: Int => Int = _ + 1
  val double: Int => Int = _ * 2
  val incThenDouble = compose(double, inc)
  println(s"compose(double, inc)(5) = ${incThenDouble(5)}")

  val add = curry[Int, Int, Int](_ + _)
  val add10 = add(10)
  println(s"add10(5) = ${add10(5)}")

  val quadruple = twice(double)
  println(s"quadruple(3) = ${quadruple(3)}")

  val result = pipeline(1, List(_ + 1, _ * 3, _ - 2))
  println(s"pipeline(1, [+1, *3, -2]) = $result")

  val fns: List[Int => String] = List(
    n => s"$n is ${if n % 2 == 0 then "even" else "odd"}",
    n => s"$n squared is ${n * n}"
  )
  fns.foreach(f => println(f(7)))
