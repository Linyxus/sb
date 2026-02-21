// Default parameters and named arguments
object DefaultParams:
  def greet(name: String = "World", greeting: String = "Hello", punctuation: String = "!"): String =
    s"$greeting, $name$punctuation"

  case class Config(
    host: String = "localhost",
    port: Int = 8080,
    debug: Boolean = false,
    maxRetries: Int = 3
  )
