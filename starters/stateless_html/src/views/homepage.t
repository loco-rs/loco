{% set environment = environment | capitalize -%}
<!DOCTYPE html>
<html>

<head>
    <title>Loco website
    </title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width">

    <style type="text/css">
        body {
            align-items: center;
            background-color: #F0E7E9;
            background-position: center center;
            background-repeat: no-repeat;
            background-size: cover;
            color: #261B23;
            display: flex;
            flex-direction: column;
            font-size: calc(0.9em + 0.5vw);
            justify-content: center;
            line-height: 1.25;
            min-height: 100vh;
            text-align: center;
        }

        nav {
            font-size: 0;
            line-height: 0;
            max-height: 480px;
            max-width: 480px;
            min-height: 286px;
            min-width: 286px;
        }
      
      

        nav a img {
            height: auto;
            max-width: 100%;
            width: 100%;
            cursor: pointer;
        }

        ul {
            bottom: 0;
            left: 0;
            list-style: none;
            margin: 0 2rem 2rem 2rem;
            position: absolute;
            right: 0;
            font-size: 16px;
        }
    </style>
</head>

<body>  
    Welcome to Logo website 
    <br/>
    <nav>
        <a href="https://loco.rs" target="_blank">
            <img alt=""
                src="https://github.com/loco-rs/loco/raw/master/media/image.png" />
        </a>
    </nav>
    <br/>
    <a href="https://loco.rs" target="_blank">loco.rs</a>

    <ul>
        <li><strong>Environment:</strong>
           {{environment}}
        </li>
        
    </ul>
</body>

</html>