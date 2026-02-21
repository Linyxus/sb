// Try/catch/finally and exception handling
object TryCatch:
  class AppException(msg: String) extends Exception(msg)
  class NotFoundException(msg: String) extends AppException(msg)
  class ValidationException(msg: String) extends AppException(msg)

  def safeDivide(a: Int, b: Int): Either[String, Int] =
    try
      Right(a / b)
    catch
      case _: ArithmeticException => Left("division by zero")

  def riskyOperation(input: String): String =
    try
      if input.isEmpty then throw NotFoundException("empty input")
      if input.length > 100 then throw ValidationException("input too long")
      input.toUpperCase
    catch
      case e: NotFoundException => s"not found: ${e.getMessage}"
      case e: ValidationException => s"invalid: ${e.getMessage}"
      case e: Exception => s"unexpected: ${e.getMessage}"
    finally
      println("cleanup done")
