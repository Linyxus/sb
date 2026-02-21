// Higher-kinded types and type lambdas
trait Functor[F[_]]:
  extension [A](fa: F[A])
    def fmap[B](f: A => B): F[B]

trait Monad[F[_]] extends Functor[F]:
  def pure[A](a: A): F[A]
  extension [A](fa: F[A])
    def flatMap[B](f: A => F[B]): F[B]
    def fmap[B](f: A => B): F[B] = fa.flatMap(a => pure(f(a)))

object HigherKinded:
  given Monad[Option] with
    def pure[A](a: A): Option[A] = Some(a)
    extension [A](fa: Option[A])
      def flatMap[B](f: A => Option[B]): Option[B] = fa match
        case Some(a) => f(a)
        case None => None

  given Monad[List] with
    def pure[A](a: A): List[A] = List(a)
    extension [A](fa: List[A])
      def flatMap[B](f: A => List[B]): List[B] = fa.flatMap(f)
