function tokenize(expression) {
    const tokens = [];
    const cleaned = expression.replace(/\s+/g, '');
    let i = 0;

    while (i < cleaned.length) {
        const char = cleaned[i];
        if (/[0-9.]/.test(char)) {
            let number = char;
            i++;
            while (i < cleaned.length && /[0-9.]/.test(cleaned[i])) {
                number += cleaned[i];
                i++;
            }
            if ((number.match(/\./g) ?? []).length > 1)
                return null;
            tokens.push(number);
            continue;
        }

        if ('+-*/()'.includes(char)) {
            tokens.push(char);
            i++;
            continue;
        }

        return null;
    }

    return tokens;
}

function precedence(operator) {
    if (operator === '+' || operator === '-')
        return 1;
    if (operator === '*' || operator === '/')
        return 2;
    return 0;
}

function toRpn(tokens) {
    const output = [];
    const operators = [];

    for (const token of tokens) {
        if (/^[0-9.]+$/.test(token)) {
            output.push(token);
            continue;
        }

        if (token === '(') {
            operators.push(token);
            continue;
        }

        if (token === ')') {
            while (operators.length > 0 && operators[operators.length - 1] !== '(')
                output.push(operators.pop());
            if (operators.pop() !== '(')
                return null;
            continue;
        }

        while (
            operators.length > 0 &&
            operators[operators.length - 1] !== '(' &&
            precedence(operators[operators.length - 1]) >= precedence(token)
        )
            output.push(operators.pop());
        operators.push(token);
    }

    while (operators.length > 0) {
        const op = operators.pop();
        if (op === '(' || op === ')')
            return null;
        output.push(op);
    }

    return output;
}

function evaluateRpn(rpn) {
    const stack = [];

    for (const token of rpn) {
        if (/^[0-9.]+$/.test(token)) {
            stack.push(Number(token));
            continue;
        }

        if (stack.length < 2)
            return null;

        const right = stack.pop();
        const left = stack.pop();
        if (!Number.isFinite(left) || !Number.isFinite(right))
            return null;

        if (token === '+')
            stack.push(left + right);
        else if (token === '-')
            stack.push(left - right);
        else if (token === '*')
            stack.push(left * right);
        else if (token === '/') {
            if (right === 0)
                return null;
            stack.push(left / right);
        } else {
            return null;
        }
    }

    if (stack.length !== 1 || !Number.isFinite(stack[0]))
        return null;

    return stack[0];
}

export function shouldHandleCalculatorQuery(query) {
    const q = (query ?? '').trim();
    return q.length > 0 && /^[0-9+\-*/().\s]+$/.test(q) && /[0-9]/.test(q);
}

export function evaluateExpression(query) {
    if (!shouldHandleCalculatorQuery(query))
        return null;

    const tokens = tokenize(query);
    if (!tokens)
        return null;

    const rpn = toRpn(tokens);
    if (!rpn)
        return null;

    const value = evaluateRpn(rpn);
    if (!Number.isFinite(value))
        return null;

    const rounded = Math.round(value * 1_000_000) / 1_000_000;
    return String(rounded);
}

export class CalculatorProvider {
    getResults(query) {
        const value = evaluateExpression(query);
        if (value === null)
            return [];

        return [{
            kind: 'utility',
            id: `calc:${query}`,
            primaryText: value,
            secondaryText: `Calculator • ${query}`,
            execute: () => {},
        }];
    }
}
