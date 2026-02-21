// Inline methods and transparent inline
object InlineMethod:
  inline def assertPositive(x: Int): Int =
    if x <= 0 then throw IllegalArgumentException(s"Expected positive, got $x")
    else x

  inline def choose[A](inline cond: Boolean, a: A, b: A): A =
    if cond then a else b

  transparent inline def typeNameOf[A]: String =
    inline compiletime.erasedValue[A] match
      case _: Int => "Int"
      case _: String => "String"
      case _ => "Other"
