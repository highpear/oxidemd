# Math Sample

This file checks inline and display math rendering in several layouts.

## Inline Math In Paragraphs

Inline math should blend into normal text, such as $e^{i\pi} + 1 = 0$.

This paragraph mixes short formulas like $a+b$, fractions like $\frac{1}{n}$, and exponents like $x^2 + y^2$ in one wrapped sentence to make baseline issues easier to spot.

Longer inline expressions should still feel readable inside a sentence, for example $\sum_{k=1}^{n} k = \frac{n(n+1)}{2}$ and $\int_0^1 x^2\,dx = \frac{1}{3}$.

## Inline Math In Other Blocks

> Blockquotes should keep inline math aligned, such as $\alpha + \beta + \gamma$ and $\sqrt{x^2 + y^2}$.

- Lists should handle inline math like $f(x)=x^3$.
- Ordered items should also work with $P(A \mid B)=\frac{P(A \cap B)}{P(B)}$.

## Inline Math In Tables

| Case | Example | Notes |
| --- | --- | --- |
| Simple | $a^2 + b^2 = c^2$ | Checks centered inline layout |
| Fraction | $\frac{dy}{dx}$ | Checks taller inline math |
| Sum | $\sum_{i=0}^{n} i$ | Checks width estimation |

## Display Math

Display math should render as its own preview block:

$$
a^2 + b^2 = c^2
$$

Larger expressions should also remain readable:

$$
\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}
$$

$$
\mathrm{Var}(X) = \mathrm{E}[X^2] - \left(\mathrm{E}[X]\right)^2
$$

## Escaping

Escaped dollars should stay as regular text: \$5.
