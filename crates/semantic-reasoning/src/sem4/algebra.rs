use std::collections::{BTreeMap, BTreeSet};

use super::model::{Equality, Expr, OperatorDefinition, Rational};

type Monomial = BTreeMap<String, u32>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Polynomial {
    pub terms: BTreeMap<Monomial, Rational>,
}

impl Rational {
    pub fn new(numerator: i64, denominator: i64) -> Result<Self, String> {
        if denominator == 0 {
            return Err("DIVISION_BY_ZERO".to_string());
        }
        let sign = if denominator < 0 { -1 } else { 1 };
        let numerator = numerator
            .checked_mul(sign)
            .ok_or_else(|| "RATIONAL_OVERFLOW".to_string())?;
        let denominator = denominator
            .checked_mul(sign)
            .ok_or_else(|| "RATIONAL_OVERFLOW".to_string())?;
        let divisor = gcd(numerator.unsigned_abs(), denominator.unsigned_abs()) as i64;
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    pub const fn integer(value: i64) -> Self {
        Self {
            numerator: value,
            denominator: 1,
        }
    }

    pub const fn zero() -> Self {
        Self::integer(0)
    }

    pub const fn one() -> Self {
        Self::integer(1)
    }

    pub fn is_zero(self) -> bool {
        self.numerator == 0
    }

    pub fn checked_add(self, other: Self) -> Result<Self, String> {
        let left = self
            .numerator
            .checked_mul(other.denominator)
            .ok_or_else(|| "RATIONAL_OVERFLOW".to_string())?;
        let right = other
            .numerator
            .checked_mul(self.denominator)
            .ok_or_else(|| "RATIONAL_OVERFLOW".to_string())?;
        let numerator = left
            .checked_add(right)
            .ok_or_else(|| "RATIONAL_OVERFLOW".to_string())?;
        let denominator = self
            .denominator
            .checked_mul(other.denominator)
            .ok_or_else(|| "RATIONAL_OVERFLOW".to_string())?;
        Self::new(numerator, denominator)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, String> {
        self.checked_add(other.checked_neg()?)
    }

    pub fn checked_mul(self, other: Self) -> Result<Self, String> {
        Self::new(
            self.numerator
                .checked_mul(other.numerator)
                .ok_or_else(|| "RATIONAL_OVERFLOW".to_string())?,
            self.denominator
                .checked_mul(other.denominator)
                .ok_or_else(|| "RATIONAL_OVERFLOW".to_string())?,
        )
    }

    pub fn checked_div(self, other: Self) -> Result<Self, String> {
        if other.is_zero() {
            return Err("DIVISION_BY_ZERO".to_string());
        }
        Self::new(
            self.numerator
                .checked_mul(other.denominator)
                .ok_or_else(|| "RATIONAL_OVERFLOW".to_string())?,
            self.denominator
                .checked_mul(other.numerator)
                .ok_or_else(|| "RATIONAL_OVERFLOW".to_string())?,
        )
    }

    pub fn checked_neg(self) -> Result<Self, String> {
        Self::new(
            self.numerator
                .checked_neg()
                .ok_or_else(|| "RATIONAL_OVERFLOW".to_string())?,
            self.denominator,
        )
    }

    pub fn checked_pow(self, exponent: u32) -> Result<Self, String> {
        let mut result = Self::one();
        for _ in 0..exponent {
            result = result.checked_mul(self)?;
        }
        Ok(result)
    }
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

pub fn rational(value: i64) -> Expr {
    Expr::Rational(Rational::integer(value))
}

pub fn variable(name: impl Into<String>) -> Expr {
    Expr::Variable(name.into())
}

pub fn add(left: Expr, right: Expr) -> Expr {
    Expr::Add(Box::new(left), Box::new(right))
}

pub fn subtract(left: Expr, right: Expr) -> Expr {
    Expr::Subtract(Box::new(left), Box::new(right))
}

pub fn multiply(left: Expr, right: Expr) -> Expr {
    Expr::Multiply(Box::new(left), Box::new(right))
}

pub fn divide(left: Expr, right: Expr) -> Expr {
    Expr::Divide(Box::new(left), Box::new(right))
}

pub fn negate(value: Expr) -> Expr {
    Expr::Negate(Box::new(value))
}

pub fn power(base: Expr, exponent: u32) -> Expr {
    Expr::Power(Box::new(base), exponent)
}

impl Polynomial {
    pub fn zero() -> Self {
        Self {
            terms: BTreeMap::new(),
        }
    }

    pub fn constant(value: Rational) -> Self {
        let mut result = Self::zero();
        if !value.is_zero() {
            result.terms.insert(BTreeMap::new(), value);
        }
        result
    }

    pub fn variable(name: String) -> Self {
        let mut monomial = BTreeMap::new();
        monomial.insert(name, 1);
        let mut terms = BTreeMap::new();
        terms.insert(monomial, Rational::one());
        Self { terms }
    }

    pub fn add(&self, other: &Self) -> Result<Self, String> {
        let mut result = self.clone();
        for (monomial, coefficient) in &other.terms {
            let current = result
                .terms
                .get(monomial)
                .copied()
                .unwrap_or_else(Rational::zero);
            let updated = current.checked_add(*coefficient)?;
            if updated.is_zero() {
                result.terms.remove(monomial);
            } else {
                result.terms.insert(monomial.clone(), updated);
            }
        }
        Ok(result)
    }

    pub fn negate(&self) -> Result<Self, String> {
        let mut result = Self::zero();
        for (monomial, coefficient) in &self.terms {
            result
                .terms
                .insert(monomial.clone(), coefficient.checked_neg()?);
        }
        Ok(result)
    }

    pub fn subtract(&self, other: &Self) -> Result<Self, String> {
        self.add(&other.negate()?)
    }

    pub fn multiply(&self, other: &Self) -> Result<Self, String> {
        let mut result = Self::zero();
        for (left_monomial, left_coefficient) in &self.terms {
            for (right_monomial, right_coefficient) in &other.terms {
                let mut monomial = left_monomial.clone();
                for (variable, exponent) in right_monomial {
                    let prior = monomial.get(variable).copied().unwrap_or_default();
                    monomial.insert(
                        variable.clone(),
                        prior
                            .checked_add(*exponent)
                            .ok_or_else(|| "POWER_OVERFLOW".to_string())?,
                    );
                }
                let coefficient = left_coefficient.checked_mul(*right_coefficient)?;
                let current = result
                    .terms
                    .get(&monomial)
                    .copied()
                    .unwrap_or_else(Rational::zero);
                let updated = current.checked_add(coefficient)?;
                if updated.is_zero() {
                    result.terms.remove(&monomial);
                } else {
                    result.terms.insert(monomial, updated);
                }
            }
        }
        Ok(result)
    }

    pub fn divide_constant(&self, divisor: Rational) -> Result<Self, String> {
        if divisor.is_zero() {
            return Err("DIVISION_BY_ZERO".to_string());
        }
        let mut result = Self::zero();
        for (monomial, coefficient) in &self.terms {
            result
                .terms
                .insert(monomial.clone(), coefficient.checked_div(divisor)?);
        }
        Ok(result)
    }

    pub fn power(&self, exponent: u32) -> Result<Self, String> {
        let mut result = Self::constant(Rational::one());
        for _ in 0..exponent {
            result = result.multiply(self)?;
        }
        Ok(result)
    }

    pub fn constant_value(&self) -> Option<Rational> {
        if self.terms.is_empty() {
            return Some(Rational::zero());
        }
        if self.terms.len() == 1 {
            self.terms.get(&BTreeMap::new()).copied()
        } else {
            None
        }
    }

    pub fn coefficient(&self, variable: &str, exponent: u32) -> Rational {
        let mut monomial = BTreeMap::new();
        if exponent > 0 {
            monomial.insert(variable.to_string(), exponent);
        }
        self.terms
            .get(&monomial)
            .copied()
            .unwrap_or_else(Rational::zero)
    }

    pub fn univariate_degree(&self, variable: &str) -> Result<u32, String> {
        let mut degree = 0;
        for monomial in self.terms.keys() {
            if monomial.keys().any(|name| name != variable) {
                return Err("NOT_UNIVARIATE".to_string());
            }
            degree = degree.max(monomial.get(variable).copied().unwrap_or_default());
        }
        Ok(degree)
    }

    pub fn to_expr(&self) -> Expr {
        if self.terms.is_empty() {
            return rational(0);
        }
        let mut terms = Vec::new();
        for (monomial, coefficient) in &self.terms {
            let mut factors = Vec::new();
            if *coefficient != Rational::one() || monomial.is_empty() {
                factors.push(Expr::Rational(*coefficient));
            }
            for (name, exponent) in monomial {
                let value = if *exponent == 1 {
                    variable(name)
                } else {
                    power(variable(name), *exponent)
                };
                factors.push(value);
            }
            let term = factors
                .into_iter()
                .reduce(multiply)
                .unwrap_or_else(|| rational(1));
            terms.push(term);
        }
        terms.into_iter().reduce(add).unwrap_or_else(|| rational(0))
    }
}

pub fn polynomial(expr: &Expr) -> Result<Polynomial, String> {
    match expr {
        Expr::Rational(value) => Ok(Polynomial::constant(*value)),
        Expr::Variable(name) => Ok(Polynomial::variable(name.clone())),
        Expr::Add(left, right) => polynomial(left)?.add(&polynomial(right)?),
        Expr::Subtract(left, right) => polynomial(left)?.subtract(&polynomial(right)?),
        Expr::Multiply(left, right) => polynomial(left)?.multiply(&polynomial(right)?),
        Expr::Divide(left, right) => {
            let divisor = polynomial(right)?
                .constant_value()
                .ok_or_else(|| "NON_CONSTANT_DIVISOR".to_string())?;
            polynomial(left)?.divide_constant(divisor)
        }
        Expr::Negate(value) => polynomial(value)?.negate(),
        Expr::Power(base, exponent) => polynomial(base)?.power(*exponent),
        Expr::Apply { .. } => Err("UNEXPANDED_OPERATOR_DEFINITION".to_string()),
    }
}

pub fn equivalent(left: &Expr, right: &Expr) -> Result<bool, String> {
    Ok(polynomial(left)? == polynomial(right)?)
}

pub fn equality_holds(equality: &Equality) -> Result<bool, String> {
    equivalent(&equality.left, &equality.right)
}

pub fn normalize(expr: &Expr) -> Result<Expr, String> {
    Ok(polynomial(expr)?.to_expr())
}

pub fn substitute(expr: &Expr, substitutions: &BTreeMap<String, Expr>) -> Expr {
    match expr {
        Expr::Rational(_) => expr.clone(),
        Expr::Variable(name) => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| expr.clone()),
        Expr::Add(left, right) => add(
            substitute(left, substitutions),
            substitute(right, substitutions),
        ),
        Expr::Subtract(left, right) => subtract(
            substitute(left, substitutions),
            substitute(right, substitutions),
        ),
        Expr::Multiply(left, right) => multiply(
            substitute(left, substitutions),
            substitute(right, substitutions),
        ),
        Expr::Divide(left, right) => divide(
            substitute(left, substitutions),
            substitute(right, substitutions),
        ),
        Expr::Negate(value) => negate(substitute(value, substitutions)),
        Expr::Power(base, exponent) => power(substitute(base, substitutions), *exponent),
        Expr::Apply {
            operator_token,
            args,
        } => Expr::Apply {
            operator_token: operator_token.clone(),
            args: args
                .iter()
                .map(|arg| substitute(arg, substitutions))
                .collect(),
        },
    }
}

pub fn expand_definitions(expr: &Expr, definitions: &[OperatorDefinition]) -> Result<Expr, String> {
    match expr {
        Expr::Apply {
            operator_token,
            args,
        } => {
            let definition = definitions
                .iter()
                .find(|definition| definition.operator_token == *operator_token)
                .ok_or_else(|| "OPERATOR_DEFINITION_MISSING".to_string())?;
            if definition.parameters.len() != args.len() {
                return Err("OPERATOR_ARITY_MISMATCH".to_string());
            }
            let substitutions = definition
                .parameters
                .iter()
                .cloned()
                .zip(args.iter().cloned())
                .collect();
            expand_definitions(&substitute(&definition.body, &substitutions), definitions)
        }
        Expr::Add(left, right) => Ok(add(
            expand_definitions(left, definitions)?,
            expand_definitions(right, definitions)?,
        )),
        Expr::Subtract(left, right) => Ok(subtract(
            expand_definitions(left, definitions)?,
            expand_definitions(right, definitions)?,
        )),
        Expr::Multiply(left, right) => Ok(multiply(
            expand_definitions(left, definitions)?,
            expand_definitions(right, definitions)?,
        )),
        Expr::Divide(left, right) => Ok(divide(
            expand_definitions(left, definitions)?,
            expand_definitions(right, definitions)?,
        )),
        Expr::Negate(value) => Ok(negate(expand_definitions(value, definitions)?)),
        Expr::Power(base, exponent) => Ok(power(expand_definitions(base, definitions)?, *exponent)),
        _ => Ok(expr.clone()),
    }
}

pub fn evaluate(expr: &Expr, assignments: &BTreeMap<String, Rational>) -> Result<Rational, String> {
    let substitutions = assignments
        .iter()
        .map(|(name, value)| (name.clone(), Expr::Rational(*value)))
        .collect();
    polynomial(&substitute(expr, &substitutions))?
        .constant_value()
        .ok_or_else(|| "UNBOUND_VARIABLE".to_string())
}

pub fn shift(expr: &Expr, variable_name: &str, amount: i64) -> Expr {
    let substitutions = BTreeMap::from([(
        variable_name.to_string(),
        add(variable(variable_name), rational(amount)),
    )]);
    substitute(expr, &substitutions)
}

pub fn synthesize_recurrence_candidate(
    base: Expr,
    delta: &Expr,
    index_variable: &str,
) -> Result<(Expr, Vec<Rational>), String> {
    let delta_polynomial = polynomial(delta)?;
    let degree = delta_polynomial.univariate_degree(index_variable)?;
    let mut coefficients = vec![Rational::zero(); degree as usize + 2];
    for power_index in (0..=degree).rev() {
        let mut residual = delta_polynomial.coefficient(index_variable, power_index);
        for candidate_power in (power_index + 2)..=(degree + 1) {
            let multiplier = Rational::integer(binomial(candidate_power, power_index) as i64);
            residual = residual
                .checked_sub(coefficients[candidate_power as usize].checked_mul(multiplier)?)?;
        }
        coefficients[(power_index + 1) as usize] =
            residual.checked_div(Rational::integer((power_index + 1) as i64))?;
    }
    let mut candidate = base;
    for (exponent, coefficient) in coefficients.iter().enumerate().skip(1) {
        if coefficient.is_zero() {
            continue;
        }
        candidate = add(
            candidate,
            multiply(
                Expr::Rational(*coefficient),
                power(variable(index_variable), exponent as u32),
            ),
        );
    }
    Ok((normalize(&candidate)?, coefficients))
}

fn binomial(n: u32, k: u32) -> u64 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut result = 1_u64;
    for index in 0..k {
        result = result * u64::from(n - index) / u64::from(index + 1);
    }
    result
}

pub fn primitive_cost(expr: &Expr) -> usize {
    match expr {
        Expr::Rational(_) | Expr::Variable(_) => 1,
        Expr::Negate(value) => 1 + primitive_cost(value),
        Expr::Power(base, exponent) => 1 + primitive_cost(base) * (*exponent as usize).max(1),
        Expr::Add(left, right)
        | Expr::Subtract(left, right)
        | Expr::Multiply(left, right)
        | Expr::Divide(left, right) => 1 + primitive_cost(left) + primitive_cost(right),
        Expr::Apply { args, .. } => 1 + args.iter().map(primitive_cost).sum::<usize>(),
    }
}

pub fn variables(expr: &Expr) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    collect_variables(expr, &mut result);
    result
}

fn collect_variables(expr: &Expr, result: &mut BTreeSet<String>) {
    match expr {
        Expr::Variable(name) => {
            result.insert(name.clone());
        }
        Expr::Add(left, right)
        | Expr::Subtract(left, right)
        | Expr::Multiply(left, right)
        | Expr::Divide(left, right) => {
            collect_variables(left, result);
            collect_variables(right, result);
        }
        Expr::Negate(value) | Expr::Power(value, _) => collect_variables(value, result),
        Expr::Apply { args, .. } => {
            for arg in args {
                collect_variables(arg, result);
            }
        }
        Expr::Rational(_) => {}
    }
}

pub fn display_expr(expr: &Expr) -> String {
    match expr {
        Expr::Rational(value) if value.denominator == 1 => value.numerator.to_string(),
        Expr::Rational(value) => format!("{}/{}", value.numerator, value.denominator),
        Expr::Variable(name) => name.clone(),
        Expr::Add(left, right) => format!("({}+{})", display_expr(left), display_expr(right)),
        Expr::Subtract(left, right) => format!("({}-{})", display_expr(left), display_expr(right)),
        Expr::Multiply(left, right) => format!("({}*{})", display_expr(left), display_expr(right)),
        Expr::Divide(left, right) => format!("({}/{})", display_expr(left), display_expr(right)),
        Expr::Negate(value) => format!("(-{})", display_expr(value)),
        Expr::Power(base, exponent) => format!("({}^{exponent})", display_expr(base)),
        Expr::Apply {
            operator_token,
            args,
        } => format!(
            "{}({})",
            operator_token,
            args.iter().map(display_expr).collect::<Vec<_>>().join(",")
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn rational_arithmetic_and_divide_by_zero_are_checked() {
        let half = Rational::new(1, 2).expect("half");
        let third = Rational::new(1, 3).expect("third");
        assert_eq!(
            half.checked_add(third).expect("sum"),
            Rational::new(5, 6).expect("five sixths")
        );
        assert!(half.checked_div(Rational::zero()).is_err());
    }

    #[test]
    fn symbolic_substitution_and_polynomial_equivalence_are_exact() {
        let expression = multiply(add(variable("x"), rational(2)), variable("x"));
        let expanded = add(
            power(variable("x"), 2),
            multiply(rational(2), variable("x")),
        );
        assert!(equivalent(&expression, &expanded).expect("equivalence"));
        let value = evaluate(
            &expression,
            &BTreeMap::from([("x".to_string(), Rational::integer(3))]),
        )
        .expect("value");
        assert_eq!(value, Rational::integer(15));
    }

    #[test]
    fn recurrence_synthesis_is_verified_by_symbolic_difference() {
        let delta = add(multiply(rational(3), variable("n")), rational(2));
        let (candidate, _) =
            synthesize_recurrence_candidate(rational(7), &delta, "n").expect("candidate");
        assert!(equivalent(
            &subtract(shift(&candidate, "n", 1), candidate.clone()),
            &delta,
        )
        .expect("difference"));
        let base = substitute(
            &candidate,
            &BTreeMap::from([("n".to_string(), rational(0))]),
        );
        assert!(equivalent(&base, &rational(7)).expect("base"));
    }
}
