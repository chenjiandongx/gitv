<!DOCTYPE html>
<html lang="en">

<head>
    <meta charset="UTF-8">
    <title>{{ title }}</title>
    {%- for dep in dependencies %}
    <script src="{{ dep }}"></script>
    {%- endfor %}
</head>

<body>
    <div>
        <canvas id="{{ chart_id }}" width="{{ width }}" height="{{ height }}"></canvas>
    </div>
    <script>
        {%- for reg in register %}
        {{ reg | safe }}
        {%- endfor %}
        const myChart = new Chart(
            document.getElementById('{{ chart_id }}'),
            {{ config }}
    );
    </script>
</body>

</html>
