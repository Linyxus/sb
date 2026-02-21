// Context functions and context parameters
object ContextFunctions:
  case class Logger(prefix: String):
    def log(msg: String): Unit = println(s"[$prefix] $msg")

  type Logged[A] = Logger ?=> A

  def withLogging[A](prefix: String)(body: Logged[A]): A =
    body(using Logger(prefix))

  def info(msg: String): Logged[Unit] =
    summon[Logger].log(msg)

  def compute(x: Int): Logged[Int] =
    info(s"computing with $x")
    val result = x * x
    info(s"result is $result")
    result
