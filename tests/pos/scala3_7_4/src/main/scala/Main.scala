import scala.compiletime.ops.int.*

type Factorial[N <: Int] <: Int = N match
  case 0 => 1
  case _ => N * Factorial[N - 1]

inline def factorial[N <: Int]: Int = scala.compiletime.constValue[Factorial[N]]

@main def app(): Unit =
  println(s"5! = ${factorial[5]}")
  println(s"0! = ${factorial[0]}")
  println(s"3! = ${factorial[3]}")
