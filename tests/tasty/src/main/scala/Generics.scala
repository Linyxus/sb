// Type parameters, variance, bounds
class Box[+A](val value: A):
  def map[B](f: A => B): Box[B] = Box(f(value))
  def flatMap[B](f: A => Box[B]): Box[B] = f(value)

trait Converter[-A, +B]:
  def convert(a: A): B

class UpperBounded[A <: AnyRef](val ref: A)

class Container[F[_]](val wrapped: F[Int])
